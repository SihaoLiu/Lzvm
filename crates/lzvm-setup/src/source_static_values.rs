use std::collections::BTreeMap;

use lzvm_pil::{
    evaluate_fixed_file_template_value_expression_with_values, FixedFileTemplateValue,
    SourceProgram,
};

pub(crate) fn source_scalar_constant_values(
    program: &SourceProgram,
    row_count: u64,
) -> BTreeMap<String, FixedFileTemplateValue> {
    let mut values = BTreeMap::from([
        (
            "BITS".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count.trailing_zeros())),
        ),
        (
            "N".to_owned(),
            FixedFileTemplateValue::Integer(i128::from(row_count)),
        ),
    ]);
    let declarations = program
        .modules
        .iter()
        .flat_map(|module| module.constants.iter())
        .collect::<Vec<_>>();
    let mut resolved = vec![false; declarations.len()];

    loop {
        let mut progressed = false;
        for (index, declaration) in declarations.iter().enumerate() {
            if resolved[index] {
                continue;
            }
            if !declaration.array_dims.is_empty() || values.contains_key(&declaration.name) {
                resolved[index] = true;
                progressed = true;
                continue;
            }
            let Some(expression) = declaration.initializer_expression.as_ref() else {
                continue;
            };
            let Some(value) =
                evaluate_fixed_file_template_value_expression_with_values(expression, &values)
            else {
                continue;
            };
            values.insert(declaration.name.clone(), value);
            resolved[index] = true;
            progressed = true;
        }
        if !progressed {
            break;
        }
    }

    values
}
