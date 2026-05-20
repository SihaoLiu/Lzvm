use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{FixedFileTemplateValue, SourceProgram, SourceProgramModule};

use crate::source_scalar_slots::SourceScalarSlots;

pub(crate) struct SourceTemplateLoweringContext<'a> {
    pub(crate) program: &'a SourceProgram,
    pub(crate) module: &'a SourceProgramModule,
    pub(crate) scalar_slots: &'a SourceScalarSlots,
    pub(crate) fixed_columns: &'a BTreeSet<String>,
    pub(crate) constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
}
