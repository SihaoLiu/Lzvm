use std::fmt;

use lzvm_artifacts::framed_stdin::{parse_framed_stdin_chunks, FramedStdinError};
use num_bigint::BigUint;

use crate::guest_machine::{
    GuestFcallError, GuestFcallHandler, GuestFcallParam, GuestFcallRequest, GuestFcallResponse,
    GuestMachineMemory,
};
use crate::guest_memory::GuestMemoryError;
use crate::secp256k1_host::{
    biguint_to_limbs, limbs_to_biguint, mod_inv, secp256k1_double_scalar_mul,
    secp256k1_field_modulus, secp256k1_order, Secp256k1Error, SecpPoint,
};

pub const ZISK_INPUT_ADDRESS: u64 = 0x4000_0000;
pub const ZISK_SECP256K1_FP_INV_FCALL_ID: u16 = 1;
pub const ZISK_SECP256K1_FN_INV_FCALL_ID: u16 = 2;
pub const ZISK_SECP256K1_FP_SQRT_FCALL_ID: u16 = 3;
pub const ZISK_MSB_POS_256_FCALL_ID: u16 = 4;
pub const ZISK_SECP256K1_ECDSA_VERIFY_FCALL_ID: u16 = 20;
pub const ZISK_INPUT_READY_FCALL_ID: u16 = 22;
const ZISK_INPUT_PREFIX_BYTES: usize = 8;
const ZISK_INPUT_MIN_READY_ADDRESS: u64 = ZISK_INPUT_ADDRESS + ZISK_INPUT_PREFIX_BYTES as u64 - 1;
const ZISK_FCALL_PARAM_WORDS: [usize; 16] =
    [1, 2, 4, 8, 12, 16, 20, 24, 28, 32, 48, 64, 80, 96, 128, 256];
const SECP256K1_G: [u64; 8] = [
    0x59f2_815b_16f8_1798,
    0x029b_fcdb_2dce_28d9,
    0x55a0_6295_ce87_0b07,
    0x79be_667e_f9dc_bbac,
    0x9c47_d08f_fb10_d4b8,
    0xfd17_b448_a685_5419,
    0x5da4_fbfc_0e11_08a8,
    0x483a_da77_26a3_c465,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZiskInputFcallError {
    FramedStdin(FramedStdinError),
    InputLengthOverflow {
        byte_len: usize,
    },
    MissingRequiredAddressParam {
        function_id: u16,
        param_count: usize,
    },
    UnexpectedRequiredAddressPort {
        port: u8,
    },
    RequiredAddressBeforeInput {
        required_address: u64,
    },
    RequiredAddressBeforeFramedStdin {
        required_address: u64,
    },
    RequiredAddressBeyondInput {
        required_address: u64,
        available_end: u64,
    },
    MissingFunctionParams {
        function_id: u16,
        expected: usize,
        found: usize,
    },
    UnexpectedFunctionParamPort {
        function_id: u16,
        param_index: usize,
        expected_port: u8,
        port: u8,
    },
    InvalidMsbPosInputCount {
        count: u64,
    },
    ZeroMsbPosInput,
    NonInvertibleScalar,
    UnsupportedFunction {
        function_id: u16,
    },
    Memory(GuestMemoryError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZiskInputFcallHandler {
    input_image: Option<Vec<u8>>,
    input_image_len: usize,
}

impl ZiskInputFcallHandler {
    pub fn new(framed_stdin: &[u8]) -> Result<Self, ZiskInputFcallError> {
        parse_framed_stdin_chunks(framed_stdin).map_err(ZiskInputFcallError::FramedStdin)?;
        let byte_len = framed_stdin
            .len()
            .checked_add(ZISK_INPUT_PREFIX_BYTES)
            .ok_or(ZiskInputFcallError::InputLengthOverflow {
                byte_len: framed_stdin.len(),
            })?;
        let mut input_image = Vec::with_capacity(byte_len);
        input_image.resize(ZISK_INPUT_PREFIX_BYTES, 0);
        input_image.extend_from_slice(framed_stdin);
        let _ = u64::try_from(input_image.len()).map_err(|_| {
            ZiskInputFcallError::InputLengthOverflow {
                byte_len: input_image.len(),
            }
        })?;

        Ok(Self {
            input_image_len: input_image.len(),
            input_image: Some(input_image),
        })
    }

    pub fn input_data_was_mapped(&self) -> bool {
        self.input_image.is_none()
    }

    #[allow(dead_code)]
    pub(crate) fn new_for_replay(
        framed_stdin: &[u8],
        input_data_was_mapped: bool,
    ) -> Result<Self, ZiskInputFcallError> {
        let mut handler = Self::new(framed_stdin)?;
        if input_data_was_mapped {
            handler.input_image = None;
        }
        Ok(handler)
    }

    fn handle_input_ready(
        &mut self,
        request: GuestFcallRequest,
        memory: &mut GuestMachineMemory,
    ) -> Result<GuestFcallResponse, ZiskInputFcallError> {
        let required_address = required_address_param(&request)?;
        if required_address < ZISK_INPUT_ADDRESS {
            return Err(ZiskInputFcallError::RequiredAddressBeforeInput { required_address });
        }
        if required_address < ZISK_INPUT_MIN_READY_ADDRESS {
            return Err(ZiskInputFcallError::RequiredAddressBeforeFramedStdin { required_address });
        }
        let available_end = self.available_end()?;
        if required_address >= available_end {
            return Err(ZiskInputFcallError::RequiredAddressBeyondInput {
                required_address,
                available_end,
            });
        }
        self.ensure_mapped(memory)?;
        Ok(GuestFcallResponse {
            results: Vec::new(),
        })
    }

    fn handle_secp256k1_ecdsa_verify(
        &mut self,
        request: GuestFcallRequest,
        memory: &GuestMachineMemory,
    ) -> Result<GuestFcallResponse, ZiskInputFcallError> {
        let pk = read_fcall_memory_param::<8>(&request, memory, 0, 3)?;
        let z = read_fcall_memory_param::<4>(&request, memory, 1, 2)?;
        let r = read_fcall_memory_param::<4>(&request, memory, 2, 2)?;
        let s = read_fcall_memory_param::<4>(&request, memory, 3, 2)?;
        if request.params.len() != 4 {
            return Err(ZiskInputFcallError::MissingFunctionParams {
                function_id: request.function_id,
                expected: 4,
                found: request.params.len(),
            });
        }
        Ok(GuestFcallResponse {
            results: secp256k1_ecdsa_verify(&pk, &z, &r, &s)?.to_vec(),
        })
    }

    fn handle_secp256k1_fp_inv(
        &mut self,
        request: GuestFcallRequest,
        memory: &GuestMachineMemory,
    ) -> Result<GuestFcallResponse, ZiskInputFcallError> {
        let value = read_fcall_memory_param::<4>(&request, memory, 0, 2)?;
        if request.params.len() != 1 {
            return Err(ZiskInputFcallError::MissingFunctionParams {
                function_id: request.function_id,
                expected: 1,
                found: request.params.len(),
            });
        }
        Ok(GuestFcallResponse {
            results: secp256k1_mod_inv(&value, secp256k1_field_modulus())?.to_vec(),
        })
    }

    fn handle_secp256k1_fn_inv(
        &mut self,
        request: GuestFcallRequest,
        memory: &GuestMachineMemory,
    ) -> Result<GuestFcallResponse, ZiskInputFcallError> {
        let value = read_fcall_memory_param::<4>(&request, memory, 0, 2)?;
        if request.params.len() != 1 {
            return Err(ZiskInputFcallError::MissingFunctionParams {
                function_id: request.function_id,
                expected: 1,
                found: request.params.len(),
            });
        }
        Ok(GuestFcallResponse {
            results: secp256k1_mod_inv(&value, secp256k1_order())?.to_vec(),
        })
    }

    fn handle_secp256k1_fp_sqrt(
        &mut self,
        request: GuestFcallRequest,
        memory: &GuestMachineMemory,
    ) -> Result<GuestFcallResponse, ZiskInputFcallError> {
        let value = read_fcall_memory_param::<4>(&request, memory, 0, 2)?;
        let Some(parity) = request.params.get(1) else {
            return Err(ZiskInputFcallError::MissingFunctionParams {
                function_id: request.function_id,
                expected: 2,
                found: request.params.len(),
            });
        };
        if parity.port != 0 {
            return Err(ZiskInputFcallError::UnexpectedFunctionParamPort {
                function_id: request.function_id,
                param_index: 1,
                expected_port: 0,
                port: parity.port,
            });
        }
        if request.params.len() != 2 {
            return Err(ZiskInputFcallError::MissingFunctionParams {
                function_id: request.function_id,
                expected: 2,
                found: request.params.len(),
            });
        }
        Ok(GuestFcallResponse {
            results: secp256k1_fp_sqrt(&value, parity.value),
        })
    }

    fn handle_msb_pos_256(
        &mut self,
        request: GuestFcallRequest,
        memory: &GuestMachineMemory,
    ) -> Result<GuestFcallResponse, ZiskInputFcallError> {
        let Some(count_param) = request.params.first() else {
            return Err(ZiskInputFcallError::MissingFunctionParams {
                function_id: request.function_id,
                expected: 1,
                found: 0,
            });
        };
        if count_param.port != 0 {
            return Err(ZiskInputFcallError::UnexpectedFunctionParamPort {
                function_id: request.function_id,
                param_index: 0,
                expected_port: 0,
                port: count_param.port,
            });
        }
        let count = usize::try_from(count_param.value).map_err(|_| {
            ZiskInputFcallError::InvalidMsbPosInputCount {
                count: count_param.value,
            }
        })?;
        if !(2..=3).contains(&count) {
            return Err(ZiskInputFcallError::InvalidMsbPosInputCount {
                count: count_param.value,
            });
        }
        if request.params.len() != count + 1 {
            return Err(ZiskInputFcallError::MissingFunctionParams {
                function_id: request.function_id,
                expected: count + 1,
                found: request.params.len(),
            });
        }

        let mut values = Vec::with_capacity(count * 4);
        for param_index in 0..count {
            values.extend_from_slice(&read_fcall_memory_param::<4>(
                &request,
                memory,
                param_index + 1,
                2,
            )?);
        }
        let (limb, bit) = msb_pos_256(&values, count)?;
        Ok(GuestFcallResponse {
            results: vec![limb as u64, bit as u64],
        })
    }

    fn ensure_mapped(
        &mut self,
        memory: &mut GuestMachineMemory,
    ) -> Result<(), ZiskInputFcallError> {
        let Some(input_image) = self.input_image.as_deref() else {
            return Ok(());
        };
        memory
            .write_or_map_initialized_range(ZISK_INPUT_ADDRESS, input_image)
            .map_err(ZiskInputFcallError::Memory)?;
        self.input_image = None;
        Ok(())
    }

    fn available_end(&self) -> Result<u64, ZiskInputFcallError> {
        let byte_len = u64::try_from(self.input_image_len).map_err(|_| {
            ZiskInputFcallError::InputLengthOverflow {
                byte_len: self.input_image_len,
            }
        })?;
        ZISK_INPUT_ADDRESS
            .checked_add(byte_len)
            .ok_or(ZiskInputFcallError::InputLengthOverflow {
                byte_len: self.input_image_len,
            })
    }
}

impl GuestFcallHandler for ZiskInputFcallHandler {
    fn handle_fcall(
        &mut self,
        request: GuestFcallRequest,
        memory: &mut GuestMachineMemory,
    ) -> Result<GuestFcallResponse, GuestFcallError> {
        match request.function_id {
            ZISK_SECP256K1_FP_INV_FCALL_ID => self
                .handle_secp256k1_fp_inv(request, memory)
                .map_err(GuestFcallError::from),
            ZISK_SECP256K1_FN_INV_FCALL_ID => self
                .handle_secp256k1_fn_inv(request, memory)
                .map_err(GuestFcallError::from),
            ZISK_SECP256K1_FP_SQRT_FCALL_ID => self
                .handle_secp256k1_fp_sqrt(request, memory)
                .map_err(GuestFcallError::from),
            ZISK_MSB_POS_256_FCALL_ID => self
                .handle_msb_pos_256(request, memory)
                .map_err(GuestFcallError::from),
            ZISK_INPUT_READY_FCALL_ID => self
                .handle_input_ready(request, memory)
                .map_err(GuestFcallError::from),
            ZISK_SECP256K1_ECDSA_VERIFY_FCALL_ID => self
                .handle_secp256k1_ecdsa_verify(request, memory)
                .map_err(GuestFcallError::from),
            function_id => Err(ZiskInputFcallError::UnsupportedFunction { function_id }.into()),
        }
    }
}

impl From<ZiskInputFcallError> for GuestFcallError {
    fn from(error: ZiskInputFcallError) -> Self {
        Self::Handler {
            message: error.to_string(),
        }
    }
}

impl From<Secp256k1Error> for ZiskInputFcallError {
    fn from(error: Secp256k1Error) -> Self {
        match error {
            Secp256k1Error::NonInvertibleScalar => Self::NonInvertibleScalar,
        }
    }
}

impl fmt::Display for ZiskInputFcallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FramedStdin(error) => write!(f, "framed stdin is invalid: {error}"),
            Self::InputLengthOverflow { byte_len } => {
                write!(f, "Zisk input length overflows: byte length {byte_len}")
            }
            Self::MissingRequiredAddressParam {
                function_id,
                param_count,
            } => write!(
                f,
                "Zisk input-ready free-call {function_id} requires one parameter, found {param_count}"
            ),
            Self::UnexpectedRequiredAddressPort { port } => write!(
                f,
                "Zisk input-ready free-call parameter port is invalid: {port}"
            ),
            Self::RequiredAddressBeforeInput { required_address } => write!(
                f,
                "Zisk input-ready required address {required_address:#x} is before input memory"
            ),
            Self::RequiredAddressBeforeFramedStdin { required_address } => write!(
                f,
                "Zisk input-ready required address {required_address:#x} is before framed stdin"
            ),
            Self::RequiredAddressBeyondInput {
                required_address,
                available_end,
            } => write!(
                f,
                "Zisk input-ready required address {required_address:#x} is outside available input ending at {available_end:#x}"
            ),
            Self::MissingFunctionParams {
                function_id,
                expected,
                found,
            } => write!(
                f,
                "Zisk free-call {function_id} requires {expected} parameters, found {found}"
            ),
            Self::UnexpectedFunctionParamPort {
                function_id,
                param_index,
                expected_port,
                port,
            } => write!(
                f,
                "Zisk free-call {function_id} parameter {param_index} port is invalid: expected {expected_port}, found {port}"
            ),
            Self::InvalidMsbPosInputCount { count } => {
                write!(f, "Zisk msb-position input count is invalid: {count}")
            }
            Self::ZeroMsbPosInput => write!(f, "Zisk msb-position input is zero"),
            Self::NonInvertibleScalar => write!(f, "Zisk secp256k1 scalar is not invertible"),
            Self::UnsupportedFunction { function_id } => {
                write!(f, "Zisk free-call function id is unsupported: {function_id}")
            }
            Self::Memory(error) => write!(f, "Zisk input memory map failed: {error}"),
        }
    }
}

impl std::error::Error for ZiskInputFcallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FramedStdin(error) => Some(error),
            Self::Memory(error) => Some(error),
            Self::InputLengthOverflow { .. }
            | Self::MissingRequiredAddressParam { .. }
            | Self::UnexpectedRequiredAddressPort { .. }
            | Self::RequiredAddressBeforeInput { .. }
            | Self::RequiredAddressBeforeFramedStdin { .. }
            | Self::RequiredAddressBeyondInput { .. }
            | Self::MissingFunctionParams { .. }
            | Self::UnexpectedFunctionParamPort { .. }
            | Self::InvalidMsbPosInputCount { .. }
            | Self::ZeroMsbPosInput
            | Self::NonInvertibleScalar
            | Self::UnsupportedFunction { .. } => None,
        }
    }
}

fn read_fcall_memory_param<const N: usize>(
    request: &GuestFcallRequest,
    memory: &GuestMachineMemory,
    param_index: usize,
    expected_port: u8,
) -> Result<[u64; N], ZiskInputFcallError> {
    let Some(param) = request.params.get(param_index) else {
        return Err(ZiskInputFcallError::MissingFunctionParams {
            function_id: request.function_id,
            expected: param_index + 1,
            found: request.params.len(),
        });
    };
    if param.port != expected_port {
        return Err(ZiskInputFcallError::UnexpectedFunctionParamPort {
            function_id: request.function_id,
            param_index,
            expected_port,
            port: param.port,
        });
    }
    debug_assert_eq!(ZISK_FCALL_PARAM_WORDS[usize::from(param.port)], N);
    read_u64_words(memory, param.value).map_err(ZiskInputFcallError::Memory)
}

fn read_u64_words<const N: usize>(
    memory: &GuestMachineMemory,
    address: u64,
) -> Result<[u64; N], GuestMemoryError> {
    let mut bytes = vec![0_u8; N * 8];
    memory.read_range_into(address, &mut bytes)?;
    let mut words = [0_u64; N];
    for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(8)) {
        *word = u64::from_le_bytes(chunk.try_into().expect("word chunk is exactly 8 bytes"));
    }
    Ok(words)
}

fn secp256k1_ecdsa_verify(
    pk: &[u64; 8],
    z: &[u64; 4],
    r: &[u64; 4],
    s: &[u64; 4],
) -> Result<[u64; 8], ZiskInputFcallError> {
    let order = secp256k1_order();
    let s_inv =
        mod_inv(&limbs_to_biguint(s), order).ok_or(ZiskInputFcallError::NonInvertibleScalar)?;
    let u1 = (limbs_to_biguint(z) * &s_inv) % order;
    let u2 = (limbs_to_biguint(r) * &s_inv) % order;
    let point = secp256k1_double_scalar_mul(
        &biguint_to_limbs::<4>(&u1),
        &SecpPoint::from_limbs(&SECP256K1_G),
        &biguint_to_limbs::<4>(&u2),
        &SecpPoint::from_limbs(pk),
    )
    .map_err(ZiskInputFcallError::from)?;
    Ok(point.to_limbs())
}

fn secp256k1_mod_inv(value: &[u64; 4], modulus: &BigUint) -> Result<[u64; 4], ZiskInputFcallError> {
    mod_inv(&limbs_to_biguint(value), modulus)
        .map(|inverse| biguint_to_limbs::<4>(&inverse))
        .ok_or(ZiskInputFcallError::NonInvertibleScalar)
}

fn secp256k1_fp_sqrt(value: &[u64; 4], parity: u64) -> Vec<u64> {
    let modulus = secp256k1_field_modulus();
    let exponent = (modulus + BigUint::from(1_u8)) >> 2;
    let value = limbs_to_biguint(value);
    let mut sqrt = value.modpow(&exponent, modulus);
    let square = (&sqrt * &sqrt) % modulus;
    let mut results = Vec::with_capacity(5);
    if square != value {
        let non_residue = BigUint::from(3_u8);
        let adjusted = (&value * non_residue) % modulus;
        let sqrt = adjusted.modpow(&exponent, modulus);
        results.push(0);
        results.extend_from_slice(&biguint_to_limbs::<4>(&sqrt));
        return results;
    }
    let sqrt_limbs = biguint_to_limbs::<4>(&sqrt);
    if (sqrt_limbs[0] & 1) != parity {
        sqrt = (modulus - sqrt) % modulus;
    }
    results.push(1);
    results.extend_from_slice(&biguint_to_limbs::<4>(&sqrt));
    results
}

fn msb_pos_256(values: &[u64], count: usize) -> Result<(usize, usize), ZiskInputFcallError> {
    debug_assert!(values.len() >= count * 4);
    for limb in (0..4).rev() {
        let mut max_word = 0_u64;
        for value_index in 0..count {
            max_word = max_word.max(values[value_index * 4 + limb]);
        }
        if max_word != 0 {
            return Ok((limb, 63 - max_word.leading_zeros() as usize));
        }
    }
    Err(ZiskInputFcallError::ZeroMsbPosInput)
}

fn required_address_param(request: &GuestFcallRequest) -> Result<u64, ZiskInputFcallError> {
    let [GuestFcallParam { port, value }] = request.params.as_slice() else {
        return Err(ZiskInputFcallError::MissingRequiredAddressParam {
            function_id: request.function_id,
            param_count: request.params.len(),
        });
    };
    if *port != 0 {
        return Err(ZiskInputFcallError::UnexpectedRequiredAddressPort { port: *port });
    }
    Ok(*value)
}
