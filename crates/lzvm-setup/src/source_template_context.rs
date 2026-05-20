use std::collections::{BTreeMap, BTreeSet};

use lzvm_pil::{FixedFileTemplateValue, SourceProgram, SourceProgramModule};

use crate::source_key_directory::SourceCommitmentSlot;

pub(crate) struct SourceTemplateLoweringContext<'a> {
    pub(crate) program: &'a SourceProgram,
    pub(crate) module: &'a SourceProgramModule,
    pub(crate) commitment_slots: &'a BTreeMap<String, SourceCommitmentSlot>,
    pub(crate) fixed_columns: &'a BTreeSet<String>,
    pub(crate) constant_values: &'a BTreeMap<String, FixedFileTemplateValue>,
}
