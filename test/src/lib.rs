use convert::*;
use raw::*;
use wasm_bindgen::prelude::wasm_bindgen;

pub(crate) mod convert;
pub(crate) mod raw;

#[macro_export]
macro_rules! js_assert_eq {
    ($a:expr, $b:expr, $msg:expr) => {
        if !$a.eq(&$b) {
            wasm_bindgen::throw_str(&format!(
                "Assertion failed because {:?} != {:?} {}",
                $a, $b, $msg
            ));
        }
    };
    ($a:expr, $b:expr) => {
        js_assert_eq!($a, $b, "")
    };
}

#[wasm_bindgen(js_name = runTests)]
pub async fn run_tests() {
    can_handle_invalid_paths().await;
    can_run_task_bytes().await;
    can_limit_tasks_bytes().await;
    can_schedule_task_bytes().await;
    can_run_task().await;
    can_limit_tasks().await;
    can_schedule_task().await;
    can_use_iter_ext().await;
}
