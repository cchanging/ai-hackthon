// SPDX-License-Identifier: MPL-2.0

#![no_std]

use ostd::{
    irq::{
        DisabledLocalIrqGuard, IrqLine, register_bottom_half_handler_l1,
        register_bottom_half_handler_l2,
    },
    sync::{
        LocalIrqDisabled, Mutex, PreemptDisabled, RwLock, RwMutex, SpinLock, SpinLockGuard,
        WriteIrqDisabled,
    },
};

static LOCK_A: SpinLock<u32> = SpinLock::new(0);
static LOCK_B: SpinLock<u32> = SpinLock::new(0);
static CYCLE_LOCK_A: SpinLock<u32> = SpinLock::new(0);
static CYCLE_LOCK_B: SpinLock<u32> = SpinLock::new(0);
static HELPER_LOCK_A: SpinLock<u32> = SpinLock::new(0);
static HELPER_LOCK_B: SpinLock<u32> = SpinLock::new(0);
static BRANCH_LOCK_A: SpinLock<u32> = SpinLock::new(0);
static BRANCH_LOCK_B: SpinLock<u32> = SpinLock::new(0);
static RETURN_LOCK_A: SpinLock<u32> = SpinLock::new(0);
static RETURN_LOCK_B: SpinLock<u32> = SpinLock::new(0);
static BH_LOCK_A: SpinLock<u32> = SpinLock::new(0);
static BH_LOCK_B: SpinLock<u32> = SpinLock::new(0);
static BH_GUARD_MOVE_LOCK: SpinLock<u32> = SpinLock::new(0);
static IRQ_DISABLED_LOCK: SpinLock<u32, LocalIrqDisabled> = SpinLock::new(0);
static IRQ_CONFLICT_LOCK: SpinLock<u32> = SpinLock::new(0);
static DEP_SRC_LOCK: SpinLock<u32> = SpinLock::new(0);
static DEP_DST_LOCK: SpinLock<u32> = SpinLock::new(0);
static CONFIGURED_IRQ_LOCK: SpinLock<u32> = SpinLock::new(0);
static CONFIGURED_SOFTIRQ_LOCK: SpinLock<u32> = SpinLock::new(0);
static IRQ_DISABLED_SHARED_LOCK: SpinLock<u32, LocalIrqDisabled> = SpinLock::new(0);
static PROPAGATED_HELPER_LOCK: SpinLock<u32> = SpinLock::new(0);
static WRAPPED_CALLBACK_LOCK: SpinLock<u32> = SpinLock::new(0);
static WRAPPED_CALLBACK_ARG_LOCK: SpinLock<u32> = SpinLock::new(0);
static JOIN_MERGE_IRQ_LOCK: SpinLock<u32> = SpinLock::new(0);
static RW_LOCK: RwLock<u32, WriteIrqDisabled> = RwLock::new(0);
static ATOMIC_SPIN_LOCK: SpinLock<u32> = SpinLock::new(0);
static ATOMIC_RW_LOCK: RwLock<u32> = RwLock::new(0);
static ATOMIC_CALLER_LOCK: SpinLock<u32> = SpinLock::new(0);
static SLEEP_MUTEX: Mutex<u32> = Mutex::new(0);
static SLEEP_RW_MUTEX: RwMutex<u32> = RwMutex::new(0);

struct LockHolder {
    inner: SpinLock<u32>,
}

struct TempHolder {
    left: SpinLock<u32>,
    right: SpinLock<u32>,
}

enum VariantLock {
    First(SpinLock<u32>),
    Second,
}

impl LockHolder {
    fn lock_inner(&self) {
        let _guard = self.inner.lock();
    }
}

pub fn deadlock_cycle_one() {
    let _guard_a = CYCLE_LOCK_A.lock();
    let _guard_b = CYCLE_LOCK_B.lock();
}

pub fn deadlock_cycle_two() {
    let _guard_b = CYCLE_LOCK_B.lock();
    let _guard_a = CYCLE_LOCK_A.lock();
}

pub fn deadlock_args_one(lock_a: &SpinLock<u32>, lock_b: &SpinLock<u32>) {
    let _guard_a = lock_a.lock();
    let _guard_b = lock_b.lock();
}

pub fn deadlock_args_two(lock_a: &SpinLock<u32>, lock_b: &SpinLock<u32>) {
    let _guard_b = lock_b.lock();
    let _guard_a = lock_a.lock();
}

fn lock_one_arg(lock: &SpinLock<u32>) {
    let _guard = lock.lock();
}

fn return_lock<'a>(lock: &'a SpinLock<u32>) -> SpinLockGuard<'a, u32, PreemptDisabled> {
    lock.lock()
}

pub fn helper_cycle_one() {
    let _guard_a = HELPER_LOCK_A.lock();
    lock_one_arg(&HELPER_LOCK_B);
}

pub fn helper_cycle_two() {
    let _guard_b = HELPER_LOCK_B.lock();
    lock_one_arg(&HELPER_LOCK_A);
}

pub fn branch_dependent_cycle(cond: bool) {
    if cond {
        let _guard_a = BRANCH_LOCK_A.lock();
        let _guard_b = BRANCH_LOCK_B.lock();
    } else {
        let _guard_b = BRANCH_LOCK_B.lock();
        let _guard_a = BRANCH_LOCK_A.lock();
    }
}

pub fn returned_guard_path() {
    let _guard_a = return_lock(&RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn explicit_drop_then_relock(lock: &SpinLock<u32>) {
    let guard = lock.lock();
    drop(guard);
    let _guard = lock.lock();
}

pub fn aa_self_deadlock(lock: &SpinLock<u32>) {
    let _guard = lock.lock();
    let _guard_again = lock.lock();
}

pub fn task_irq_conflict() {
    irq_conflict_worker();
}

pub fn register_top_half(line: &mut IrqLine) {
    line.on_active(|_| top_half_entry());
}

pub fn register_configured_irq() {
    register_configured_irq_entry(configured_irq_worker);
}

pub fn register_configured_softirq() {
    register_configured_softirq_entry(configured_softirq_worker);
}

fn register_configured_irq_entry(callback: fn()) {
    let _ = callback;
}

fn register_configured_softirq_entry(callback: fn()) {
    let _ = callback;
}

fn top_half_entry() {
    irq_conflict_worker();
    task_dependency_edge();
    shared_irq_disabled_worker();
}
fn irq_conflict_worker() {
    let _guard = IRQ_CONFLICT_LOCK.lock();
}

pub fn register_bottom_halves() {
    register_bottom_half_handler_l1(bottom_half_l1);
    register_bottom_half_handler_l2(bottom_half_l2);
}

fn bottom_half_l1(irq_guard: DisabledLocalIrqGuard, _irq: u8) -> DisabledLocalIrqGuard {
    let _guard = IRQ_DISABLED_LOCK.lock();
    bottom_half_l1_worker();
    bottom_half_l1_guard_move(irq_guard)
}

fn bottom_half_l1_worker() {
    let _guard = BH_LOCK_A.lock();
}

fn bottom_half_l1_guard_move(irq_guard: DisabledLocalIrqGuard) -> DisabledLocalIrqGuard {
    let moved_guard = irq_guard;
    let _guard = BH_GUARD_MOVE_LOCK.lock();
    moved_guard
}

fn bottom_half_l2(_irq: u8) {
    bottom_half_l2_worker();
}

fn bottom_half_l2_worker() {
    let _guard = BH_LOCK_B.lock();
}

pub fn task_disable_irq_lock() {
    let _guard = LOCK_A.disable_irq().lock();
}

pub fn task_disable_irq_then_plain_lock() {
    {
        let _guard = LOCK_A.disable_irq().lock();
    }
    let _guard = LOCK_B.lock();
}

pub fn task_branch_irq_merge(cond: bool) {
    if cond {
        let _guard = LOCK_A.disable_irq().lock();
    }
    let _guard = JOIN_MERGE_IRQ_LOCK.lock();
}

pub fn task_irq_disabled_shared_lock() {
    shared_irq_disabled_worker();
}

pub fn task_dependency_edge() {
    let _guard_src = DEP_SRC_LOCK.disable_irq().lock();
    dependency_target_worker();
}

pub fn task_dependency_target() {
    dependency_target_worker();
}

fn dependency_target_worker() {
    let _guard = DEP_DST_LOCK.lock();
}

fn configured_irq_worker() {
    let _guard = CONFIGURED_IRQ_LOCK.lock();
}

pub fn task_softirq_conflict() {
    configured_softirq_worker();
}

fn configured_softirq_worker() {
    let _guard = CONFIGURED_SOFTIRQ_LOCK.lock();
}

fn shared_irq_disabled_worker() {
    let _guard = IRQ_DISABLED_SHARED_LOCK.lock();
}

fn propagated_helper_lock() {
    let _guard = PROPAGATED_HELPER_LOCK.lock();
}

fn invoke_callback(callback: fn()) {
    callback();
}

fn invoke_callback_alias(callback: fn()) {
    let cb = callback;
    cb();
}

fn invoke_callback_while_holding(callback: fn()) {
    let _guard = HELPER_LOCK_A.lock();
    callback();
}

fn outer_invoke_callback(callback: fn()) {
    invoke_callback_alias(callback);
}

fn outer_invoke_callback_with_lock(callback: fn(&SpinLock<u32>), lock: &SpinLock<u32>) {
    invoke_callback_with_lock_alias(callback, lock);
}

fn outer_invoke_callback_return_lock<'a>(
    callback: fn(&'a SpinLock<u32>) -> SpinLockGuard<'a, u32, PreemptDisabled>,
    lock: &'a SpinLock<u32>,
) -> SpinLockGuard<'a, u32, PreemptDisabled> {
    invoke_callback_return_lock_alias(callback, lock)
}

fn invoke_callback_with_lock(callback: fn(&SpinLock<u32>), lock: &SpinLock<u32>) {
    callback(lock);
}

fn invoke_callback_with_lock_alias(callback: fn(&SpinLock<u32>), lock: &SpinLock<u32>) {
    let cb = callback;
    cb(lock);
}

fn invoke_callback_return_lock<'a>(
    callback: fn(&'a SpinLock<u32>) -> SpinLockGuard<'a, u32, PreemptDisabled>,
    lock: &'a SpinLock<u32>,
) -> SpinLockGuard<'a, u32, PreemptDisabled> {
    callback(lock)
}

fn invoke_callback_return_lock_alias<'a>(
    callback: fn(&'a SpinLock<u32>) -> SpinLockGuard<'a, u32, PreemptDisabled>,
    lock: &'a SpinLock<u32>,
) -> SpinLockGuard<'a, u32, PreemptDisabled> {
    let cb = callback;
    cb(lock)
}

fn invoke_callback_return_lock_with_lock<'a>(
    callback: fn(&'a SpinLock<u32>) -> SpinLockGuard<'a, u32, PreemptDisabled>,
    lock: &'a SpinLock<u32>,
) -> SpinLockGuard<'a, u32, PreemptDisabled> {
    callback(lock)
}

fn invoke_callback_return_lock_with_lock_alias<'a>(
    callback: fn(&'a SpinLock<u32>) -> SpinLockGuard<'a, u32, PreemptDisabled>,
    lock: &'a SpinLock<u32>,
) -> SpinLockGuard<'a, u32, PreemptDisabled> {
    let cb = callback;
    cb(lock)
}

fn wrapped_callback_target() {
    let _guard = WRAPPED_CALLBACK_LOCK.lock();
}

fn wrapped_callback_with_lock(lock: &SpinLock<u32>) {
    let _guard = lock.lock();
}

pub fn call_propagated_helper_plain() {
    propagated_helper_lock();
}

pub fn call_propagated_helper_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    propagated_helper_lock();
}

pub fn call_indirect_propagated_helper_plain() {
    let helper = propagated_helper_lock;
    helper();
}

pub fn call_indirect_propagated_helper_irq_disabled() {
    let helper = propagated_helper_lock;
    let _guard = LOCK_A.disable_irq().lock();
    helper();
}

pub fn indirect_returned_guard_path() {
    let helper = return_lock;
    let _guard_a = helper(&RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn call_wrapped_callback_plain() {
    invoke_callback(wrapped_callback_target);
}

pub fn call_wrapped_callback_alias_plain() {
    invoke_callback_alias(wrapped_callback_target);
}

pub fn call_wrapped_callback_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    invoke_callback(wrapped_callback_target);
}

pub fn call_wrapped_callback_alias_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    invoke_callback_alias(wrapped_callback_target);
}

pub fn call_outer_wrapped_callback_plain() {
    outer_invoke_callback(wrapped_callback_target);
}

pub fn call_outer_wrapped_callback_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    outer_invoke_callback(wrapped_callback_target);
}

pub fn call_wrapped_callback_while_holding_plain() {
    invoke_callback_while_holding(wrapped_callback_target);
}

pub fn call_wrapped_callback_while_holding_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    invoke_callback_while_holding(wrapped_callback_target);
}

pub fn call_wrapped_callback_with_held_lock() {
    let _guard = LOCK_B.lock();
    invoke_callback(wrapped_callback_target);
}

pub fn call_wrapped_callback_with_held_lock_irq_disabled() {
    let _guard = LOCK_B.disable_irq().lock();
    invoke_callback(wrapped_callback_target);
}

pub fn call_wrapped_callback_with_lock_plain() {
    invoke_callback_with_lock(wrapped_callback_with_lock, &WRAPPED_CALLBACK_ARG_LOCK);
}

pub fn call_wrapped_callback_with_lock_alias_plain() {
    invoke_callback_with_lock_alias(wrapped_callback_with_lock, &WRAPPED_CALLBACK_ARG_LOCK);
}

pub fn call_wrapped_callback_with_lock_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    invoke_callback_with_lock(wrapped_callback_with_lock, &WRAPPED_CALLBACK_ARG_LOCK);
}

pub fn call_wrapped_callback_with_lock_alias_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    invoke_callback_with_lock_alias(wrapped_callback_with_lock, &WRAPPED_CALLBACK_ARG_LOCK);
}

pub fn call_outer_wrapped_callback_with_lock_plain() {
    outer_invoke_callback_with_lock(wrapped_callback_with_lock, &WRAPPED_CALLBACK_ARG_LOCK);
}

pub fn call_outer_wrapped_callback_with_lock_irq_disabled() {
    let _guard = LOCK_A.disable_irq().lock();
    outer_invoke_callback_with_lock(wrapped_callback_with_lock, &WRAPPED_CALLBACK_ARG_LOCK);
}

pub fn returned_guard_through_callback_wrapper() {
    let _guard_a = invoke_callback_return_lock(return_lock, &RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn returned_guard_through_callback_wrapper_irq_disabled() {
    let _guard_irq = LOCK_A.disable_irq().lock();
    let _guard_a = invoke_callback_return_lock(return_lock, &RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn returned_guard_through_callback_wrapper_with_lock() {
    let _guard_a = invoke_callback_return_lock_with_lock(return_lock, &RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn returned_guard_through_callback_wrapper_with_lock_irq_disabled() {
    let _guard_irq = LOCK_A.disable_irq().lock();
    let _guard_a = invoke_callback_return_lock_with_lock(return_lock, &RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn returned_guard_through_callback_wrapper_with_lock_alias() {
    let _guard_a = invoke_callback_return_lock_with_lock_alias(return_lock, &RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn returned_guard_through_callback_wrapper_alias() {
    let _guard_a = invoke_callback_return_lock_alias(return_lock, &RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn returned_guard_through_outer_callback_wrapper() {
    let _guard_a = outer_invoke_callback_return_lock(return_lock, &RETURN_LOCK_A);
    let _guard_b = RETURN_LOCK_B.lock();
}

pub fn rw_read_path() {
    let _guard = RW_LOCK.read();
}

pub fn rw_write_path() {
    let _guard = RW_LOCK.write();
}

pub fn atomic_spin_then_mutex() {
    let _guard = ATOMIC_SPIN_LOCK.lock();
    let _sleep_guard = SLEEP_MUTEX.lock();
}

pub fn atomic_rwlock_then_rwmutex() {
    let _guard = ATOMIC_RW_LOCK.read();
    let _sleep_guard = SLEEP_RW_MUTEX.write();
}

pub fn atomic_callsite_to_mutex_helper() {
    let _guard = ATOMIC_CALLER_LOCK.lock();
    mutex_helper();
}

fn mutex_helper() {
    let _guard = SLEEP_MUTEX.lock();
}

pub fn receiver_method_path() {
    let holder = LockHolder {
        inner: SpinLock::new(0),
    };
    holder.lock_inner();
}

pub fn fn_arg_alias_path(lock: &SpinLock<u32>) {
    let alias = lock;
    let _guard = alias.lock();
}

pub fn global_ref_alias_path() {
    let lock_ref = &LOCK_A;
    let _guard = lock_ref.lock();
}

pub fn aggregate_local_path() {
    let holder = TempHolder {
        left: SpinLock::new(0),
        right: SpinLock::new(1),
    };
    let _guard = holder.right.lock();
}

pub fn downcast_projection_path() {
    let variant = VariantLock::First(SpinLock::new(0));
    match &variant {
        VariantLock::First(lock) => {
            let _guard = lock.lock();
        }
        VariantLock::Second => {}
    }
}

pub fn constant_index_projection_path() {
    let locks: [SpinLock<u32>; 2] = [SpinLock::new(0), SpinLock::new(1)];
    let _guard = locks[1].lock();
}

pub fn variable_index_projection_path(index: usize) {
    let locks: [SpinLock<u32>; 2] = [SpinLock::new(0), SpinLock::new(1)];
    let lock = &locks[index];
    let _guard = lock.lock();
}
