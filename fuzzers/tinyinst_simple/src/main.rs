use std::path::PathBuf;

use libafl::{
    bolts::{
        rands::{RandomSeed, StdRand},
        shmem::{ShMem, ShMemProvider, Win32ShMemProvider},
        tuples::tuple_list,
    },
    corpus::{CachedOnDiskCorpus, Corpus, OnDiskCorpus, Testcase},
    events::SimpleEventManager,
    feedbacks::{CrashFeedback, ListFeedback},
    inputs::BytesInput,
    monitors::SimpleMonitor,
    mutators::{havoc_mutations, StdScheduledMutator},
    observers::ListObserver,
    schedulers::RandScheduler,
    stages::StdMutationalStage,
    state::StdState,
    Fuzzer, StdFuzzer,
};
use libafl_tinyinst::executor::TinyInstExecutorBuilder;
static mut COVERAGE: Vec<u64> = vec![];

fn main() {
    // Tinyinst things
    let tinyinst_args = vec!["-instrument_module".to_string(), "test.exe".to_string()];

    // use shmem to pass testcases
    let args = vec!["test.exe".to_string(), "-m".to_string(), "@@".to_string()];

    // use file to pass testcases
    // let args = vec!["test.exe".to_string(), "-f".to_string(), "@@".to_string()];

    let observer = unsafe { ListObserver::new("cov", &mut COVERAGE) };
    let mut feedback = ListFeedback::with_observer(&observer);

    let mut shmem_provider = Win32ShMemProvider::new().unwrap();
    let input = BytesInput::new(b"bad".to_vec());
    let rand = StdRand::new();
    let mut corpus = CachedOnDiskCorpus::new(PathBuf::from("./corpus_discovered"), 64).unwrap();
    corpus
        .add(Testcase::new(input))
        .expect("error in adding corpus");
    let solutions = OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap();

    let mut objective = CrashFeedback::new();
    let mut state = StdState::new(rand, corpus, solutions, &mut feedback, &mut objective).unwrap();
    let scheduler = RandScheduler::new();
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    let monitor = SimpleMonitor::new(|x| println!("{}", x));

    let mut mgr = SimpleEventManager::new(monitor);
    let mut executor = unsafe {
        TinyInstExecutorBuilder::new()
            .tinyinst_args(tinyinst_args)
            .program_args(args)
            .shmem_provider(&mut shmem_provider)
            .persistent("test.exe".to_string(), "fuzz".to_string(), 1, 10000)
            .timeout(std::time::Duration::new(5, 0))
            .build(&mut COVERAGE, tuple_list!(observer))
            .unwrap()
    };
    let mutator = StdScheduledMutator::new(havoc_mutations());
    let mut stages = tuple_list!(StdMutationalStage::new(mutator));
    fuzzer
        .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
        .expect("error in fuzzing loop");
}
