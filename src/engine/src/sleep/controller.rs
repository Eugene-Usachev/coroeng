// use std::collections::BinaryHeap;
// use std::intrinsics::unlikely;
// use std::mem;
// use std::time::Instant;
// use crate::local_scheduler;
// use crate::sleep::SleepingCoroutine;
//
// /// The boundary between "small" and "large" amount.
// ///
// /// While the amount of sleeping coroutines less than 10_000 [`Vec`]
// /// with a complete iteration is more performance than any other solution.
// ///
// /// But if the amount of sleeping coroutines more than 10_000, we need some ordered structure for fast iteration.
// const BOUNDARY: usize = 10_000;
//
// /// Each time the number of dormant coroutines crosses the boundary,
// /// we must create a new structure and copy all dormant coroutines there.
// /// Therefore, we can get into a situation where we cross the border first once,
// /// then the second time and then the rest of it.
// /// In this case, we'll be constantly copying roughly the BOUNDARY of the elements.
// ///
// /// To avoid this, we introduce a RESIZE_COEFFICIENT that is responsible for the size at which we should do the creation of a new structure.
// ///
// /// If it is 2, we will create a new structure and copy all the elements there only if we cross the BOUNDARY/2 or BOUNDARY*2 mark.
// /// This avoids the above case.
// const RESIZE_COEFFICIENT: f32 = 1.2;
//
// const SMALL_BOUNDARY: usize = (BOUNDARY as f32 / RESIZE_COEFFICIENT) as usize;
// const LARGE_BOUNDARY: usize = (BOUNDARY as f32 * RESIZE_COEFFICIENT) as usize;
//
// trait SleepController {
//     /// Awakes coroutines that are ready to be woken up.
//     fn awake_coroutines(&mut self);
//
//     /// Registers a new sleeping coroutine.
//     fn register(&mut self, sleeping_coroutine: SleepingCoroutine);
//
//     /// Returns the amount of sleeping coroutines.
//     fn amount(&self) -> usize;
// }
//
// pub(crate) enum SleepControllerType {
//     Small,
//     Large
// }
//
// /// Based on [`Vec`]. This is the best way for a little amount of sleeping coroutines (check [`BOUNDARY`] docs for more information).
// struct SmallSleepController {
//     sleeping_coroutines: Vec<SleepingCoroutine>
// }
//
// impl SmallSleepController {
//     fn new() -> Self {
//         Self {
//             sleeping_coroutines: Vec::new()
//         }
//     }
//
//     fn from_large(large_controller: &mut LargeSleepController) -> Self {
//         let small_controller = Self {
//             sleeping_coroutines: Vec::with_capacity(SMALL_BOUNDARY)
//         };
//
//         let mut large_controller_copy = unsafe { mem::zeroed() };
//         mem::swap(&mut large_controller_copy, large_controller);
//
//         for sleeping_coroutine in large_controller_copy.sleeping_coroutines.into_iter() {
//             small_controller.sleeping_coroutines.push(sleeping_coroutine);
//         }
//
//         small_controller
//     }
// }
//
// impl SleepController for SmallSleepController {
//     fn awake_coroutines(&mut self) {
//         let local_scheduler = local_scheduler();
//         let mut new_list = Vec::with_capacity(self.amount());
//         mem::swap(&mut new_list, &mut self.sleeping_coroutines);
//         let now = Instant::now();
//         for sleeping_coroutine in new_list.into_iter() {
//             if unlikely(now >= sleeping_coroutine.execution_time) {
//                 local_scheduler.sched(sleeping_coroutine.co);
//                 continue;
//             }
//             self.sleeping_coroutines.push(sleeping_coroutine);
//         }
//     }
//
//     fn register(&mut self, sleeping_coroutine: SleepingCoroutine) {
//         self.sleeping_coroutines.push(sleeping_coroutine);
//     }
//
//     fn amount(&self) -> usize {
//         self.sleeping_coroutines.len()
//     }
// }
//
// struct LargeSleepController {
//     sleeping_coroutines: BinaryHeap<>
// }
//
// impl LargeSleepController {
//     fn from_small(small_controller: &SmallSleepController) -> Self {
//
//     }
//
//     /// Awakes coroutines that are ready to be woken up and returns the new amount of sleeping coroutines after the awaking.
//     fn awake_coroutines(&mut self) {
//
//     }
//
//     fn register(&mut self, sleeping_coroutine: SleepingCoroutine) {
//
//     }
// }
//
// pub(crate) enum AutoSleepController {
//     Small(SmallSleepController),
//     Large(LargeSleepController)
// }
//
// impl AutoSleepController {
//     pub fn new() -> Self {
//         AutoSleepController::Small(SmallSleepController::new())
//     }
//
//     #[inline(always)]
//     fn amount(&self) -> usize {
//         match self {
//             AutoSleepController::Small(controller) => {
//                 controller.amount()
//             }
//             AutoSleepController::Large(controller) => {
//                 controller.amount()
//             }
//         }
//     }
//
//     #[inline(always)]
//     fn grow(&mut self) {
//         match self {
//             AutoSleepController::Small(controller) => {
//                 let new_controller = AutoSleepController::Large(LargeSleepController::from_small(controller));
//                 *self = new_controller;
//             }
//             AutoSleepController::Large(controller) => {
//                 unreachable!();
//             }
//         }
//     }
//
//     #[inline(always)]
//     fn shrink(&mut self) {
//         match self {
//             AutoSleepController::Large(controller) => {
//                 let new_controller = AutoSleepController::Small(SmallSleepController::from_large(controller));
//                 *self = new_controller;
//             }
//             AutoSleepController::Small(controller) => {
//                 unreachable!();
//             }
//         }
//     }
//
//     #[inline(always)]
//     fn register(&mut self, sleeping_coroutine: SleepingCoroutine) {
//         if unlikely(self.amount() + 1 > LARGE_BOUNDARY) {
//             self.grow();
//         }
//         match self {
//             AutoSleepController::Small(controller) => {
//                 controller.register(sleeping_coroutine);
//             }
//             AutoSleepController::Large(controller) => {
//                 controller.register(sleeping_coroutine);
//             }
//         }
//     }
//
//     #[inline(always)]
//     fn awake_coroutines(&mut self) {
//         match self {
//             AutoSleepController::Small(controller) => {
//                 controller.awake_coroutines();
//             }
//             AutoSleepController::Large(controller) => {
//                 controller.awake_coroutines();
//             }
//         }
//
//         if unlikely(self.amount() < SMALL_BOUNDARY) {
//             self.shrink();
//         }
//     }
// }