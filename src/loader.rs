use crate::api::{yaml_free, yaml_malloc, yaml_stack_extend, yaml_strdup};
use crate::externs::strcmp;
use crate::yaml::{yaml_char_t, YamlEventData, YamlNodeData};
use crate::{
    libc, yaml_alias_data_t, yaml_document_delete, yaml_document_t, yaml_event_t, yaml_mark_t,
    yaml_node_item_t, yaml_node_pair_t, yaml_node_t, yaml_parser_parse, yaml_parser_t, PointerExt,
    YAML_COMPOSER_ERROR, YAML_MEMORY_ERROR,
};
use core::mem::{size_of, MaybeUninit};
use core::ptr::{self, addr_of_mut};

#[repr(C)]
struct loader_ctx {
    start: *mut libc::c_int,
    end: *mut libc::c_int,
    top: *mut libc::c_int,
}

/// Parse the input stream and produce the next YAML document.
///
/// Call this function subsequently to produce a sequence of documents
/// constituting the input stream.
///
/// If the produced document has no root node, it means that the document end
/// has been reached.
///
/// An application is responsible for freeing any data associated with the
/// produced document object using the yaml_document_delete() function.
///
/// An application must not alternate the calls of yaml_parser_load() with the
/// calls of yaml_parser_scan() or yaml_parser_parse(). Doing this will break
/// the parser.
pub unsafe fn yaml_parser_load(
    parser: &mut yaml_parser_t,
    document: &mut yaml_document_t,
) -> Result<(), ()> {
    let current_block: u64;
    let mut event = yaml_event_t::default();
    *document = core::mem::MaybeUninit::zeroed().assume_init();
    STACK_INIT!((*document).nodes, yaml_node_t);
    if !parser.stream_start_produced {
        if let Err(()) = yaml_parser_parse(parser, &mut event) {
            current_block = 6234624449317607669;
        } else {
            if let YamlEventData::StreamStart { .. } = &event.data {
            } else {
                panic!("expected stream start");
            }
            current_block = 7815301370352969686;
        }
    } else {
        current_block = 7815301370352969686;
    }
    if current_block != 6234624449317607669 {
        if parser.stream_end_produced {
            return Ok(());
        }
        if let Ok(()) = yaml_parser_parse(parser, &mut event) {
            if let YamlEventData::StreamEnd = &event.data {
                return Ok(());
            }
            STACK_INIT!(parser.aliases, yaml_alias_data_t);
            parser.document = document;
            if let Ok(()) = yaml_parser_load_document(parser, &mut event) {
                yaml_parser_delete_aliases(parser);
                parser.document = ptr::null_mut::<yaml_document_t>();
                return Ok(());
            }
        }
    }
    yaml_parser_delete_aliases(parser);
    yaml_document_delete(document);
    parser.document = ptr::null_mut::<yaml_document_t>();
    Err(())
}

unsafe fn yaml_parser_set_composer_error(
    parser: &mut yaml_parser_t,
    problem: &'static str,
    problem_mark: yaml_mark_t,
) -> Result<(), ()> {
    parser.error = YAML_COMPOSER_ERROR;
    parser.problem = Some(problem);
    parser.problem_mark = problem_mark;
    Err(())
}

unsafe fn yaml_parser_set_composer_error_context(
    parser: &mut yaml_parser_t,
    context: &'static str,
    context_mark: yaml_mark_t,
    problem: &'static str,
    problem_mark: yaml_mark_t,
) -> Result<(), ()> {
    parser.error = YAML_COMPOSER_ERROR;
    parser.context = Some(context);
    parser.context_mark = context_mark;
    parser.problem = Some(problem);
    parser.problem_mark = problem_mark;
    Err(())
}

unsafe fn yaml_parser_delete_aliases(parser: &mut yaml_parser_t) {
    while !STACK_EMPTY!(parser.aliases) {
        yaml_free(POP!(parser.aliases).anchor as *mut libc::c_void);
    }
    STACK_DEL!(parser.aliases);
}

unsafe fn yaml_parser_load_document(
    parser: &mut yaml_parser_t,
    event: &mut yaml_event_t,
) -> Result<(), ()> {
    let mut ctx = loader_ctx {
        start: ptr::null_mut::<libc::c_int>(),
        end: ptr::null_mut::<libc::c_int>(),
        top: ptr::null_mut::<libc::c_int>(),
    };
    if let YamlEventData::DocumentStart {
        version_directive,
        tag_directives_start,
        tag_directives_end,
        implicit,
    } = &event.data
    {
        (*parser.document).version_directive = *version_directive;
        (*parser.document).tag_directives.start = *tag_directives_start;
        (*parser.document).tag_directives.end = *tag_directives_end;
        (*parser.document).start_implicit = *implicit;
        (*parser.document).start_mark = event.start_mark;
        STACK_INIT!(ctx, libc::c_int);
        if let Err(()) = yaml_parser_load_nodes(parser, &mut ctx) {
            STACK_DEL!(ctx);
            return Err(());
        }
        STACK_DEL!(ctx);
        Ok(())
    } else {
        crate::externs::__assert_fail("event.type_ == YAML_DOCUMENT_START_EVENT", file!(), line!())
    }
}

unsafe fn yaml_parser_load_nodes(
    parser: &mut yaml_parser_t,
    ctx: &mut loader_ctx,
) -> Result<(), ()> {
    let mut event = yaml_event_t::default();
    let end_implicit;
    let end_mark;

    loop {
        yaml_parser_parse(parser, &mut event)?;
        match &event.data {
            YamlEventData::NoEvent => panic!("empty event"),
            YamlEventData::StreamStart { .. } => panic!("unexpected stream start event"),
            YamlEventData::StreamEnd => panic!("unexpected stream end event"),
            YamlEventData::DocumentStart { .. } => panic!("unexpected document start event"),
            YamlEventData::DocumentEnd { implicit } => {
                end_implicit = *implicit;
                end_mark = event.end_mark;
                break;
            }
            YamlEventData::Alias { .. } => {
                yaml_parser_load_alias(parser, &mut event, ctx)?;
            }
            YamlEventData::Scalar { .. } => {
                yaml_parser_load_scalar(parser, &mut event, ctx)?;
            }
            YamlEventData::SequenceStart { .. } => {
                yaml_parser_load_sequence(parser, &mut event, ctx)?;
            }
            YamlEventData::SequenceEnd => {
                yaml_parser_load_sequence_end(parser, &mut event, ctx)?;
            }
            YamlEventData::MappingStart { .. } => {
                yaml_parser_load_mapping(parser, &mut event, ctx)?;
            }
            YamlEventData::MappingEnd => {
                yaml_parser_load_mapping_end(parser, &mut event, ctx)?;
            }
        }
    }
    (*parser.document).end_implicit = end_implicit;
    (*parser.document).end_mark = end_mark;
    Ok(())
}

unsafe fn yaml_parser_register_anchor(
    parser: &mut yaml_parser_t,
    index: libc::c_int,
    anchor: *mut yaml_char_t,
) -> Result<(), ()> {
    if anchor.is_null() {
        return Ok(());
    }
    let data = yaml_alias_data_t {
        anchor,
        index,
        mark: (*(*parser.document)
            .nodes
            .start
            .wrapping_offset((index - 1) as isize))
        .start_mark,
    };
    let mut alias_data = parser.aliases.start;
    while alias_data != parser.aliases.top {
        if strcmp(
            (*alias_data).anchor as *mut libc::c_char,
            anchor as *mut libc::c_char,
        ) == 0
        {
            yaml_free(anchor as *mut libc::c_void);
            return yaml_parser_set_composer_error_context(
                parser,
                "found duplicate anchor; first occurrence",
                (*alias_data).mark,
                "second occurrence",
                data.mark,
            );
        }
        alias_data = alias_data.wrapping_offset(1);
    }
    PUSH!(parser.aliases, data);
    Ok(())
}

unsafe fn yaml_parser_load_node_add(
    parser: &mut yaml_parser_t,
    ctx: *mut loader_ctx,
    index: libc::c_int,
) -> Result<(), ()> {
    if STACK_EMPTY!(*ctx) {
        return Ok(());
    }
    let parent_index: libc::c_int = *(*ctx).top.wrapping_offset(-1_isize);
    let parent: *mut yaml_node_t = addr_of_mut!(
        *((*parser.document).nodes.start).wrapping_offset((parent_index - 1) as isize)
    );
    let current_block_17: u64;
    match (*parent).data {
        YamlNodeData::Sequence { ref mut items, .. } => {
            STACK_LIMIT!(parser, items)?;
            PUSH!(items, index);
        }
        YamlNodeData::Mapping { ref mut pairs, .. } => {
            let mut pair = MaybeUninit::<yaml_node_pair_t>::uninit();
            let pair = pair.as_mut_ptr();
            if !STACK_EMPTY!(pairs) {
                let p: *mut yaml_node_pair_t = pairs.top.wrapping_offset(-1_isize);
                if (*p).key != 0 && (*p).value == 0 {
                    (*p).value = index;
                    current_block_17 = 11307063007268554308;
                } else {
                    current_block_17 = 17407779659766490442;
                }
            } else {
                current_block_17 = 17407779659766490442;
            }
            match current_block_17 {
                11307063007268554308 => {}
                _ => {
                    (*pair).key = index;
                    (*pair).value = 0;
                    STACK_LIMIT!(parser, pairs)?;
                    PUSH!(pairs, *pair);
                }
            }
        }
        _ => {
            __assert!(false);
        }
    }
    Ok(())
}

unsafe fn yaml_parser_load_alias(
    parser: &mut yaml_parser_t,
    event: &mut yaml_event_t, // TODO: Take by value
    ctx: &mut loader_ctx,
) -> Result<(), ()> {
    let anchor: *mut yaml_char_t = if let YamlEventData::Alias { anchor } = &event.data {
        *anchor
    } else {
        unreachable!()
    };

    let mut alias_data: *mut yaml_alias_data_t;
    alias_data = parser.aliases.start;
    while alias_data != parser.aliases.top {
        if strcmp(
            (*alias_data).anchor as *mut libc::c_char,
            anchor as *mut libc::c_char,
        ) == 0
        {
            yaml_free(anchor as *mut libc::c_void);
            return yaml_parser_load_node_add(parser, ctx, (*alias_data).index);
        }
        alias_data = alias_data.wrapping_offset(1);
    }
    yaml_free(anchor as *mut libc::c_void);
    yaml_parser_set_composer_error(parser, "found undefined alias", (*event).start_mark)
}

unsafe fn yaml_parser_load_scalar(
    parser: &mut yaml_parser_t,
    event: &mut yaml_event_t, // TODO: Take by value
    ctx: &mut loader_ctx,
) -> Result<(), ()> {
    let (mut tag, value, length, style, anchor) = if let YamlEventData::Scalar {
        tag,
        value,
        length,
        style,
        anchor,
        ..
    } = &event.data
    {
        (*tag, *value, *length, *style, *anchor)
    } else {
        unreachable!()
    };

    let current_block: u64;
    let index: libc::c_int;
    if let Ok(()) = STACK_LIMIT!(parser, (*parser.document).nodes) {
        if tag.is_null()
            || strcmp(
                tag as *mut libc::c_char,
                b"!\0" as *const u8 as *const libc::c_char,
            ) == 0
        {
            yaml_free(tag as *mut libc::c_void);
            tag = yaml_strdup(
                b"tag:yaml.org,2002:str\0" as *const u8 as *const libc::c_char as *mut yaml_char_t,
            );
            if tag.is_null() {
                current_block = 10579931339944277179;
            } else {
                current_block = 11006700562992250127;
            }
        } else {
            current_block = 11006700562992250127;
        }
        if current_block != 10579931339944277179 {
            let node = yaml_node_t {
                data: YamlNodeData::Scalar {
                    value,
                    length,
                    style,
                },
                tag,
                start_mark: (*event).start_mark,
                end_mark: (*event).end_mark,
            };
            PUSH!((*parser.document).nodes, node);
            index = (*parser.document)
                .nodes
                .top
                .c_offset_from((*parser.document).nodes.start) as libc::c_int;
            yaml_parser_register_anchor(parser, index, anchor)?;
            return yaml_parser_load_node_add(parser, ctx, index);
        }
    }
    yaml_free(tag as *mut libc::c_void);
    yaml_free(anchor as *mut libc::c_void);
    yaml_free(value as *mut libc::c_void);
    Err(())
}

unsafe fn yaml_parser_load_sequence(
    parser: &mut yaml_parser_t,
    event: &mut yaml_event_t, // TODO: Take by value.
    ctx: &mut loader_ctx,
) -> Result<(), ()> {
    let (tag, style, anchor) = if let YamlEventData::SequenceStart {
        anchor, tag, style, ..
    } = &event.data
    {
        (*tag, *style, *anchor)
    } else {
        unreachable!()
    };

    let current_block: u64;
    struct Items {
        start: *mut yaml_node_item_t,
        end: *mut yaml_node_item_t,
        top: *mut yaml_node_item_t,
    }
    let mut items = Items {
        start: ptr::null_mut::<yaml_node_item_t>(),
        end: ptr::null_mut::<yaml_node_item_t>(),
        top: ptr::null_mut::<yaml_node_item_t>(),
    };
    let index: libc::c_int;
    let mut tag: *mut yaml_char_t = tag;
    if let Ok(()) = STACK_LIMIT!(parser, (*parser.document).nodes) {
        if tag.is_null()
            || strcmp(
                tag as *mut libc::c_char,
                b"!\0" as *const u8 as *const libc::c_char,
            ) == 0
        {
            yaml_free(tag as *mut libc::c_void);
            tag = yaml_strdup(
                b"tag:yaml.org,2002:seq\0" as *const u8 as *const libc::c_char as *mut yaml_char_t,
            );
            if tag.is_null() {
                current_block = 13474536459355229096;
            } else {
                current_block = 6937071982253665452;
            }
        } else {
            current_block = 6937071982253665452;
        }
        if current_block != 13474536459355229096 {
            STACK_INIT!(items, yaml_node_item_t);

            let node = yaml_node_t {
                data: YamlNodeData::Sequence {
                    items: crate::yaml_stack_t {
                        start: items.start,
                        end: items.end,
                        top: items.start,
                    },
                    style,
                },
                tag,
                start_mark: (*event).start_mark,
                end_mark: (*event).end_mark,
            };

            PUSH!((*parser.document).nodes, node);
            index = (*parser.document)
                .nodes
                .top
                .c_offset_from((*parser.document).nodes.start) as libc::c_int;
            yaml_parser_register_anchor(parser, index, anchor)?;
            yaml_parser_load_node_add(parser, ctx, index)?;
            STACK_LIMIT!(parser, *ctx)?;
            PUSH!(*ctx, index);
            return Ok(());
        }
    }
    yaml_free(tag as *mut libc::c_void);
    yaml_free(anchor as *mut libc::c_void);
    Err(())
}

unsafe fn yaml_parser_load_sequence_end(
    parser: &mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> Result<(), ()> {
    __assert!(((*ctx).top).c_offset_from((*ctx).start) as libc::c_long > 0_i64);
    let index: libc::c_int = *(*ctx).top.wrapping_offset(-1_isize);
    __assert!(matches!(
        (*((*parser.document).nodes.start).wrapping_offset((index - 1) as isize)).data,
        YamlNodeData::Sequence { .. }
    ));
    (*(*parser.document)
        .nodes
        .start
        .wrapping_offset((index - 1) as isize))
    .end_mark = (*event).end_mark;
    let _ = POP!(*ctx);
    Ok(())
}

unsafe fn yaml_parser_load_mapping(
    parser: &mut yaml_parser_t,
    event: &mut yaml_event_t, // TODO: take by value
    ctx: &mut loader_ctx,
) -> Result<(), ()> {
    let (tag, style, anchor) = if let YamlEventData::MappingStart {
        anchor, tag, style, ..
    } = &event.data
    {
        (*tag, *style, *anchor)
    } else {
        unreachable!()
    };

    let current_block: u64;
    struct Pairs {
        start: *mut yaml_node_pair_t,
        end: *mut yaml_node_pair_t,
        top: *mut yaml_node_pair_t,
    }
    let mut pairs = Pairs {
        start: ptr::null_mut::<yaml_node_pair_t>(),
        end: ptr::null_mut::<yaml_node_pair_t>(),
        top: ptr::null_mut::<yaml_node_pair_t>(),
    };
    let index: libc::c_int;
    let mut tag: *mut yaml_char_t = tag;
    if let Ok(()) = STACK_LIMIT!(parser, (*parser.document).nodes) {
        if tag.is_null()
            || strcmp(
                tag as *mut libc::c_char,
                b"!\0" as *const u8 as *const libc::c_char,
            ) == 0
        {
            yaml_free(tag as *mut libc::c_void);
            tag = yaml_strdup(
                b"tag:yaml.org,2002:map\0" as *const u8 as *const libc::c_char as *mut yaml_char_t,
            );
            if tag.is_null() {
                current_block = 13635467803606088781;
            } else {
                current_block = 6937071982253665452;
            }
        } else {
            current_block = 6937071982253665452;
        }
        if current_block != 13635467803606088781 {
            STACK_INIT!(pairs, yaml_node_pair_t);
            let node = yaml_node_t {
                data: YamlNodeData::Mapping {
                    pairs: crate::yaml_stack_t {
                        start: pairs.start,
                        end: pairs.end,
                        top: pairs.start,
                    },
                    style,
                },
                tag,
                start_mark: (*event).start_mark,
                end_mark: (*event).end_mark,
            };
            PUSH!((*parser.document).nodes, node);
            index = (*parser.document)
                .nodes
                .top
                .c_offset_from((*parser.document).nodes.start) as libc::c_int;
            yaml_parser_register_anchor(parser, index, anchor)?;
            yaml_parser_load_node_add(parser, ctx, index)?;
            STACK_LIMIT!(parser, *ctx)?;
            PUSH!(*ctx, index);
            return Ok(());
        }
    }
    yaml_free(tag as *mut libc::c_void);
    yaml_free(anchor as *mut libc::c_void);
    Err(())
}

unsafe fn yaml_parser_load_mapping_end(
    parser: &mut yaml_parser_t,
    event: *mut yaml_event_t,
    ctx: *mut loader_ctx,
) -> Result<(), ()> {
    __assert!(((*ctx).top).c_offset_from((*ctx).start) as libc::c_long > 0_i64);
    let index: libc::c_int = *(*ctx).top.wrapping_offset(-1_isize);
    __assert!(matches!(
        (*((*parser.document).nodes.start).wrapping_offset((index - 1) as isize)).data,
        YamlNodeData::Mapping { .. }
    ));
    (*(*parser.document)
        .nodes
        .start
        .wrapping_offset((index - 1) as isize))
    .end_mark = (*event).end_mark;
    let _ = POP!(*ctx);
    Ok(())
}
