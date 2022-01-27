use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use grba_core::scheduler::EventTag;

fn bench_scheduler(c: &mut Criterion) {
    let mut group = c.benchmark_group("Schedulers");
    for i in [2u64, 6u64].iter() {
        let mut new_sch = grba_core::scheduler::Scheduler::new();
        group.bench_with_input(BenchmarkId::new("ArrayVec", i), i, |b, i| {
            b.iter(|| {
                for j in 0..(*i) {
                    if j % 2 == 0 {
                        new_sch.schedule_event(EventTag::VBlank, (j * i * (j % 3)).into())
                    }
                }

                new_sch.pop_current().unwrap();
            });

            new_sch.clear_events()
        });
    }
    group.finish();
}

criterion_group!(benches, bench_scheduler);
criterion_main!(benches);
