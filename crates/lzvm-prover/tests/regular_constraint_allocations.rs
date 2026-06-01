use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use lzvm_artifacts::constraint_program::{ConstraintEntry, ConstraintProgram};
use lzvm_field::Felt;
use lzvm_prover::regular_constraints::{
    evaluate_regular_constraints, RegularColumnMatrix, RegularConstraintInputs,
};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

static COUNT_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_LOCK: Mutex<()> = Mutex::new(());

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn reuses_regular_constraint_row_scratch() {
    let _lock = ALLOCATION_LOCK
        .lock()
        .expect("allocation test lock poisoned");
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 0,
            last_row: 64,
            temp1_count: 1,
            temp3_count: 1,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "scratch allocation residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![3],
    };
    let fixed = vec![felt(3); 64];

    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    COUNT_ALLOCATIONS.store(true, Ordering::Relaxed);
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 64,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("regular constraint should evaluate");
    COUNT_ALLOCATIONS.store(false, Ordering::Relaxed);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].invalid_rows, Vec::new());
    assert!(
        ALLOCATION_COUNT.load(Ordering::Relaxed) < 16,
        "regular constraint evaluation should not allocate scratch per row"
    );
}

#[test]
fn skips_row_scratch_when_active_row_range_is_empty() {
    let _lock = ALLOCATION_LOCK
        .lock()
        .expect("allocation test lock poisoned");
    let program = ConstraintProgram {
        entries: vec![ConstraintEntry {
            stage: 1,
            destination_dimension: 1,
            destination_id: 0,
            first_row: 64,
            last_row: 64,
            temp1_count: 16,
            temp3_count: 16,
            ops_count: 1,
            ops_offset: 0,
            args_count: 8,
            args_offset: 0,
            intermediate: false,
            source_line: "empty row residual".to_owned(),
        }],
        ops: vec![0],
        args: vec![1, 0, 0, 0, 0, 8, 0, 0],
        numbers: vec![3],
    };
    let fixed = vec![felt(3); 8];

    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    COUNT_ALLOCATIONS.store(true, Ordering::Relaxed);
    let results = evaluate_regular_constraints(
        &program,
        RegularConstraintInputs {
            domain_size: 8,
            stage_count: 1,
            fixed_columns: RegularColumnMatrix {
                column_count: 1,
                values: &fixed,
            },
            opening_point_offsets: &[0],
            ..RegularConstraintInputs::default()
        },
    )
    .expect("empty active row range should still evaluate");
    COUNT_ALLOCATIONS.store(false, Ordering::Relaxed);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].invalid_rows, Vec::new());
    assert!(
        ALLOCATION_COUNT.load(Ordering::Relaxed) < 3,
        "empty active row ranges should not allocate row scratch"
    );
}

fn felt(value: u64) -> Felt {
    Felt::from_u64(value)
}
