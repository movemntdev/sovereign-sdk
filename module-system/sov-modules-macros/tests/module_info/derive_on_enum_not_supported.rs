use sov_modules_macros::ModuleInfo;
use sov_state::StateMap;

#[derive(ModuleInfo)]
enum TestStruct<C: sov_modules_api::Context> {
    #[state]
    TestState1(StateMap<String, String>),

    #[state]
    TestState2(C),
}

fn main() {}
