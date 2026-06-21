use num_bigint::BigUint;
use num_traits::Zero;
use tiny_keccak::keccakf;

use crate::guest_instruction::RiscvPrecompileKind;
use crate::guest_machine::{
    GuestInstructionEffects, GuestMachineError, GuestMachineMemory, GuestMachineState,
};
use crate::guest_memory::GuestMemoryError;
use crate::secp256k1_host::{
    secp256k1_point_add, secp256k1_point_double, Secp256k1Error, SecpPoint,
};

const KECCAK_STATE_BYTES: usize = 25 * 8;

pub(super) fn execute_precompile(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    kind: RiscvPrecompileKind,
    instruction_address: u64,
    operand_address: u64,
) -> Result<u64, GuestMachineError> {
    match kind {
        RiscvPrecompileKind::Keccak => {
            execute_keccak_precompile(memory, state, effects, operand_address)?;
            Ok(0)
        }
        RiscvPrecompileKind::Arith256 => {
            execute_arith256_precompile(memory, state, effects, operand_address)?;
            Ok(0)
        }
        RiscvPrecompileKind::Arith256Mod => {
            execute_arith256_mod_precompile(
                memory,
                state,
                effects,
                instruction_address,
                operand_address,
            )?;
            Ok(0)
        }
        RiscvPrecompileKind::Secp256k1Add => {
            execute_secp256k1_add_precompile(
                memory,
                state,
                effects,
                instruction_address,
                operand_address,
            )?;
            Ok(0)
        }
        RiscvPrecompileKind::Secp256k1Dbl => {
            execute_secp256k1_dbl_precompile(
                memory,
                state,
                effects,
                instruction_address,
                operand_address,
            )?;
            Ok(0)
        }
        RiscvPrecompileKind::Add256 => {
            execute_add256_precompile(memory, state, effects, operand_address)
        }
    }
}

fn execute_keccak_precompile(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    address: u64,
) -> Result<(), GuestMachineError> {
    let mut words = read_u64_words::<25>(memory, effects, address)?;
    keccakf(&mut words);
    state.clear_reservation_if_overlaps(address, KECCAK_STATE_BYTES);
    write_u64_words(memory, state, effects, address, &words)?;
    Ok(())
}

fn execute_arith256_precompile(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    params_address: u64,
) -> Result<(), GuestMachineError> {
    let params = read_u64_words::<5>(memory, effects, params_address)?;
    let a = read_u64_words::<4>(memory, effects, params[0])?;
    let b = read_u64_words::<4>(memory, effects, params[1])?;
    let c = read_u64_words::<4>(memory, effects, params[2])?;
    let result = words_to_biguint(&a) * words_to_biguint(&b) + words_to_biguint(&c);
    let low = biguint_to_words::<4>(&result);
    let high = biguint_to_words::<4>(&(result >> 256));
    write_u64_words(memory, state, effects, params[3], &low)?;
    write_u64_words(memory, state, effects, params[4], &high)?;
    Ok(())
}

fn execute_arith256_mod_precompile(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    instruction_address: u64,
    params_address: u64,
) -> Result<(), GuestMachineError> {
    let params = read_u64_words::<5>(memory, effects, params_address)?;
    let a = read_u64_words::<4>(memory, effects, params[0])?;
    let b = read_u64_words::<4>(memory, effects, params[1])?;
    let c = read_u64_words::<4>(memory, effects, params[2])?;
    let modulus = read_u64_words::<4>(memory, effects, params[3])?;
    let modulus = words_to_biguint(&modulus);
    if modulus.is_zero() {
        return Err(GuestMachineError::ZeroArith256Modulus {
            address: instruction_address,
        });
    }
    let result = (words_to_biguint(&a) * words_to_biguint(&b) + words_to_biguint(&c)) % modulus;
    let words = biguint_to_words::<4>(&result);
    write_u64_words(memory, state, effects, params[4], &words)?;
    Ok(())
}

fn execute_secp256k1_add_precompile(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    instruction_address: u64,
    params_address: u64,
) -> Result<(), GuestMachineError> {
    let params = read_u64_words::<2>(memory, effects, params_address)?;
    let p1 = read_u64_words::<8>(memory, effects, params[0])?;
    let p2 = read_u64_words::<8>(memory, effects, params[1])?;
    let result = secp256k1_point_add(&SecpPoint::from_limbs(&p1), &SecpPoint::from_limbs(&p2))
        .map_err(|error| secp256k1_precompile_error(instruction_address, error))?;
    write_u64_words(memory, state, effects, params[0], &result.to_limbs())?;
    Ok(())
}

fn execute_secp256k1_dbl_precompile(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    instruction_address: u64,
    address: u64,
) -> Result<(), GuestMachineError> {
    let p1 = read_u64_words::<8>(memory, effects, address)?;
    let result = secp256k1_point_double(&SecpPoint::from_limbs(&p1))
        .map_err(|error| secp256k1_precompile_error(instruction_address, error))?;
    write_u64_words(memory, state, effects, address, &result.to_limbs())?;
    Ok(())
}

fn secp256k1_precompile_error(address: u64, error: Secp256k1Error) -> GuestMachineError {
    match error {
        Secp256k1Error::NonInvertibleScalar => {
            GuestMachineError::NonInvertibleSecp256k1Scalar { address }
        }
    }
}

fn execute_add256_precompile(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    params_address: u64,
) -> Result<u64, GuestMachineError> {
    let params = read_u64_words::<4>(memory, effects, params_address)?;
    let a = read_u64_words::<4>(memory, effects, params[0])?;
    let b = read_u64_words::<4>(memory, effects, params[1])?;
    let (low, carry) = add256_words(a, b, params[2]);
    write_u64_words(memory, state, effects, params[3], &low)?;
    Ok(carry)
}

fn add256_words(a: [u64; 4], b: [u64; 4], carry_in: u64) -> ([u64; 4], u64) {
    let mut low = [0_u64; 4];
    let mut carry = u128::from(carry_in);
    for ((out, a), b) in low.iter_mut().zip(a).zip(b) {
        let sum = u128::from(a) + u128::from(b) + carry;
        *out = sum as u64;
        carry = sum >> 64;
    }
    (low, u64::from(carry != 0))
}

fn read_u64_words<const N: usize>(
    memory: &GuestMachineMemory,
    effects: &mut GuestInstructionEffects,
    address: u64,
) -> Result<[u64; N], GuestMemoryError> {
    let mut bytes = vec![0_u8; N * 8];
    memory.read_range_into(address, &mut bytes)?;
    let mut words = [0_u64; N];
    for (index, (word, chunk)) in words.iter_mut().zip(bytes.chunks_exact(8)).enumerate() {
        *word = u64::from_le_bytes(chunk.try_into().expect("word chunk is exactly 8 bytes"));
        effects.record_precompile_memory_read(address + index as u64 * 8, 8, *word);
    }
    Ok(words)
}

fn write_u64_words<const N: usize>(
    memory: &mut GuestMachineMemory,
    state: &mut GuestMachineState,
    effects: &mut GuestInstructionEffects,
    address: u64,
    words: &[u64; N],
) -> Result<(), GuestMemoryError> {
    let mut bytes = Vec::with_capacity(N * 8);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    state.clear_reservation_if_overlaps(address, bytes.len());
    memory.write_range(address, &bytes)?;
    for (index, word) in words.iter().enumerate() {
        effects.record_precompile_memory_write(address + index as u64 * 8, 8, *word);
    }
    Ok(())
}

fn words_to_biguint(words: &[u64]) -> BigUint {
    let mut bytes = Vec::with_capacity(words.len() * 8);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    BigUint::from_bytes_le(&bytes)
}

fn biguint_to_words<const N: usize>(value: &BigUint) -> [u64; N] {
    let mut bytes = value.to_bytes_le();
    bytes.resize(N * 8, 0);
    let mut words = [0_u64; N];
    for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(8)) {
        *word = u64::from_le_bytes(chunk.try_into().expect("word chunk is exactly 8 bytes"));
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add256_words_returns_low_limbs_and_carry() {
        let (low, carry) = add256_words([u64::MAX, u64::MAX, u64::MAX, u64::MAX], [1, 0, 0, 0], 1);

        assert_eq!(low, [1, 0, 0, 0]);
        assert_eq!(carry, 1);
    }

    #[test]
    fn add256_words_matches_biguint_reference() {
        for (a, b, carry_in) in [
            ([0, 0, 0, 0], [0, 0, 0, 0], 0),
            ([5, 6, 7, 8], [9, 10, 11, 12], 13),
            ([u64::MAX, 0, 0, 0], [1, 0, 0, 0], 0),
            (
                [u64::MAX, u64::MAX, u64::MAX, u64::MAX],
                [u64::MAX, u64::MAX, u64::MAX, u64::MAX],
                u64::MAX,
            ),
        ] {
            let result = words_to_biguint(&a) + words_to_biguint(&b) + BigUint::from(carry_in);
            let (low, carry) = add256_words(a, b, carry_in);

            assert_eq!(low, biguint_to_words::<4>(&result));
            assert_eq!(carry, u64::from(result >= (BigUint::from(1_u8) << 256)));
        }
    }
}
