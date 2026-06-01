use lzvm_pil::{Token, TokenKind};

use super::{source_helper_argument, source_helper_value_list, SourceLookupArgument};

pub(super) fn source_memory_helper_call_name(tokens: &[Token]) -> Option<&str> {
    let token = tokens.first()?;
    if token.kind != TokenKind::Identifier {
        return None;
    }
    match token.lexeme.as_str() {
        "mem_op"
        | "reg_pre_load"
        | "reg_pre_store"
        | "global_init_mem"
        | "precompiled_mem_load"
        | "precompiled_mem_store"
        | "precompiled_mem_op"
        | "precompiled_mem_proves"
        | "precompiled_mem_load_padding" => Some(token.lexeme.as_str()),
        _ => None,
    }
}

pub(super) fn source_memory_helper_lookup_line(
    line: &str,
    tokens: &[Token],
    call_name: &str,
    arguments: &[SourceLookupArgument],
) -> Option<String> {
    let id = source_helper_argument(line, tokens, arguments, "id", 0, Some("MEMORY_ID"))?;
    match call_name {
        "mem_op" => {
            let op = source_helper_argument(line, tokens, arguments, "op", 1, None)?;
            let addr = source_helper_argument(line, tokens, arguments, "addr", 2, None)?;
            let mem_step = source_helper_argument(line, tokens, arguments, "mem_step", 3, None)?;
            let bytes = source_helper_argument(line, tokens, arguments, "bytes", 4, Some("8"))?;
            let value = source_helper_argument(line, tokens, arguments, "value", 5, None)?;
            let sel = source_helper_argument(line, tokens, arguments, "sel", 6, Some("1"))?;
            let values = source_helper_value_list(&value);
            Some(format!(
                "permutation_assumes({id}, [{op}, {addr}, {mem_step}, {bytes}, {values}], sel: {sel})"
            ))
        }
        "reg_pre_load" | "reg_pre_store" => {
            let addr = source_helper_argument(line, tokens, arguments, "addr", 1, None)?;
            let prev_mem_step =
                source_helper_argument(line, tokens, arguments, "prev_mem_step", 2, None)?;
            let value = source_helper_argument(line, tokens, arguments, "value", 3, None)?;
            let sel = source_helper_argument(line, tokens, arguments, "sel", 4, Some("1"))?;
            let values = source_helper_value_list(&value);
            Some(format!(
                "permutation_proves({id}, [MEMORY_REG_OP, {addr}, {prev_mem_step}, 8, {values}], sel: {sel})"
            ))
        }
        "global_init_mem" => {
            let addr = source_helper_argument(line, tokens, arguments, "addr", 1, None)?;
            let value = source_helper_argument(line, tokens, arguments, "value", 2, None)?;
            let sel = source_helper_argument(line, tokens, arguments, "sel", 3, Some("1"))?;
            let values = source_helper_value_list(&value);
            Some(format!(
                "direct_global_update_assumes({id}, [MEMORY_REG_OP, {addr}, 0, 8, {values}], sel: {sel})"
            ))
        }
        "precompiled_mem_load" => {
            let addr = source_helper_argument(line, tokens, arguments, "addr", 1, None)?;
            let main_step = source_helper_argument(line, tokens, arguments, "main_step", 2, None)?;
            let value = source_helper_argument(line, tokens, arguments, "value", 3, None)?;
            let sel = source_helper_argument(line, tokens, arguments, "sel", 4, Some("1"))?;
            let mem_step = source_precompiled_mem_step(&main_step, "0");
            let values = source_helper_value_list(&value);
            Some(format!(
                "permutation_assumes({id}, [MEMORY_LOAD_OP, {addr}, {mem_step}, 8, {values}], sel: {sel})"
            ))
        }
        "precompiled_mem_store" => {
            let addr = source_helper_argument(line, tokens, arguments, "addr", 1, None)?;
            let main_step = source_helper_argument(line, tokens, arguments, "main_step", 2, None)?;
            let value = source_helper_argument(line, tokens, arguments, "value", 3, None)?;
            let sel = source_helper_argument(line, tokens, arguments, "sel", 4, Some("1"))?;
            let mem_step = source_precompiled_mem_step(&main_step, "1");
            let values = source_helper_value_list(&value);
            Some(format!(
                "permutation_assumes({id}, [MEMORY_STORE_OP, {addr}, {mem_step}, 8, {values}], sel: {sel})"
            ))
        }
        "precompiled_mem_op" => {
            let addr = source_helper_argument(line, tokens, arguments, "addr", 1, None)?;
            let main_step = source_helper_argument(line, tokens, arguments, "main_step", 2, None)?;
            let value = source_helper_argument(line, tokens, arguments, "value", 3, None)?;
            let sel = source_helper_argument(line, tokens, arguments, "sel", 4, Some("1"))?;
            let is_write = source_precompiled_mem_is_write(line, tokens, arguments)?;
            let op = format!("{is_write} * (MEMORY_STORE_OP - MEMORY_LOAD_OP) + MEMORY_LOAD_OP");
            let mem_step = source_precompiled_mem_step(&main_step, &is_write);
            let values = source_helper_value_list(&value);
            Some(format!(
                "permutation_assumes({id}, [{op}, {addr}, {mem_step}, 8, {values}], sel: {sel})"
            ))
        }
        "precompiled_mem_proves" => {
            let addr = source_helper_argument(line, tokens, arguments, "addr", 1, None)?;
            let main_step = source_helper_argument(line, tokens, arguments, "main_step", 2, None)?;
            let value = source_helper_argument(line, tokens, arguments, "value", 3, None)?;
            let sel = source_helper_argument(line, tokens, arguments, "sel", 4, Some("1"))?;
            let is_write = source_precompiled_mem_is_write(line, tokens, arguments)?;
            let op = format!("{is_write} * (MEMORY_STORE_OP - MEMORY_LOAD_OP) + MEMORY_LOAD_OP");
            let mem_step = source_precompiled_mem_step(&main_step, &is_write);
            let values = source_helper_value_list(&value);
            Some(format!(
                "permutation_proves({id}, [{op}, {addr}, {mem_step}, 8, {values}], sel: {sel})"
            ))
        }
        "precompiled_mem_load_padding" => {
            let padding = source_helper_argument(line, tokens, arguments, "padding", 1, Some("0"))?;
            let mem_step = source_precompiled_mem_step("0", "0");
            Some(format!(
                "permutation_proves({id}, [MEMORY_LOAD_OP, 0, {mem_step}, 8, 0, 0], sel: {padding})"
            ))
        }
        _ => None,
    }
}

fn source_precompiled_mem_is_write(
    line: &str,
    tokens: &[Token],
    arguments: &[SourceLookupArgument],
) -> Option<String> {
    let positional_count = arguments
        .iter()
        .filter(|argument| argument.name.is_none())
        .count();
    let positional_index = if positional_count >= 7 { 6 } else { 5 };
    source_helper_argument(
        line,
        tokens,
        arguments,
        "is_write",
        positional_index,
        Some("0"),
    )
}

fn source_precompiled_mem_step(main_step: &str, is_write: &str) -> String {
    format!("RESERVED_MEM_STEPS + MAX_MEM_STEPS_PER_MAIN_STEP * {main_step} + {is_write} + 2")
}
