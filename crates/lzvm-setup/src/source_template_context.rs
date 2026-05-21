use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{FixedFileTemplateValue, SourceProgram, SourceProgramModule, Token};

use crate::source_scalar_slots::SourceScalarSlots;
use crate::source_static_values::SourceTemplateConstantValueCache;

pub(crate) struct SourceTemplateLoweringContext<'a> {
    pub(crate) program: &'a SourceProgram,
    pub(crate) module: &'a SourceProgramModule,
    pub(crate) tokens: &'a [Token],
    pub(crate) scalar_slots: &'a SourceScalarSlots,
    pub(crate) opening_points: &'a [i64],
    pub(crate) fixed_columns: &'a BTreeSet<String>,
    pub(crate) active_templates: &'a BTreeSet<String>,
    pub(crate) constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
    pub(crate) template_values: &'a SourceTemplateConstantValueCache,
}
