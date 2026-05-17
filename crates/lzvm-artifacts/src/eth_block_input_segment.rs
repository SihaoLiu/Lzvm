use crate::eth_block_input::{
    encode_eth_block_input, parse_eth_block_input, EthBlockInput, EthBlockInputError,
};

pub const ETH_BLOCK_INPUT_SEGMENT_ID: u32 = 10_013;

pub fn encode_eth_block_input_segment(
    value: &EthBlockInput,
) -> Result<Vec<u8>, EthBlockInputError> {
    encode_eth_block_input(value)
}

pub fn parse_eth_block_input_segment(bytes: &[u8]) -> Result<EthBlockInput, EthBlockInputError> {
    parse_eth_block_input(bytes)
}
