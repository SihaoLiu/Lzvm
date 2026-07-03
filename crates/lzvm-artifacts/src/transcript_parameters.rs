const SUPPORTED_TRANSCRIPT_ARITIES: [u32; 2] = [2, 4];

pub(crate) fn is_supported_transcript_arity(arity: u32) -> bool {
    SUPPORTED_TRANSCRIPT_ARITIES.contains(&arity)
}

pub(crate) fn is_supported_transcript_arity_u64(arity: u64) -> bool {
    u32::try_from(arity)
        .ok()
        .is_some_and(is_supported_transcript_arity)
}
