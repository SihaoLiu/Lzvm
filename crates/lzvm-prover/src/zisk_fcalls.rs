use std::fmt;

use lzvm_artifacts::framed_stdin::{parse_framed_stdin_chunks, FramedStdinError};

use crate::guest_machine::{
    GuestFcallError, GuestFcallHandler, GuestFcallParam, GuestFcallRequest, GuestFcallResponse,
    GuestMachineMemory,
};
use crate::guest_memory::GuestMemoryError;

pub const ZISK_INPUT_ADDRESS: u64 = 0x4000_0000;
pub const ZISK_INPUT_READY_FCALL_ID: u16 = 22;
const ZISK_INPUT_PREFIX_BYTES: usize = 8;
const ZISK_INPUT_MIN_READY_ADDRESS: u64 = ZISK_INPUT_ADDRESS + ZISK_INPUT_PREFIX_BYTES as u64 - 1;

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

    fn ensure_mapped(
        &mut self,
        memory: &mut GuestMachineMemory,
    ) -> Result<(), ZiskInputFcallError> {
        let Some(input_image) = self.input_image.take() else {
            return Ok(());
        };
        memory
            .map_initialized_range(ZISK_INPUT_ADDRESS, input_image)
            .map_err(ZiskInputFcallError::Memory)?;
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
        if request.function_id != ZISK_INPUT_READY_FCALL_ID {
            return Err(ZiskInputFcallError::UnsupportedFunction {
                function_id: request.function_id,
            }
            .into());
        }
        self.handle_input_ready(request, memory)
            .map_err(GuestFcallError::from)
    }
}

impl From<ZiskInputFcallError> for GuestFcallError {
    fn from(error: ZiskInputFcallError) -> Self {
        Self::Handler {
            message: error.to_string(),
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
            | Self::UnsupportedFunction { .. } => None,
        }
    }
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
