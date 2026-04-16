use std::collections::HashMap;

use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rumors::{Action, Tree};

fn make_values(n: usize) -> Vec<Bytes> {
    (0..n)
        .map(|i| {
            let b = i.to_le_bytes();
            Bytes::copy_from_slice(&b)
        })
        .collect()
}

fn make_inserts(values: &[Bytes]) -> Vec<Action<String>> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| Action::Insert {
            party: "".to_string(),
            version: i as u64,
            value: v.clone(),
        })
        .collect()
}

fn bench_act_insert_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("act/insert_batch_into_empty");

    for n in [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000] {
        let values = make_values(n);
        let actions = make_inserts(&values);

        group.measurement_time(if n >= 100_000 {
            std::time::Duration::from_secs(15)
        } else {
            std::time::Duration::from_secs(5)
        });

        group.bench_with_input(BenchmarkId::from_parameter(n), &actions, |b, actions| {
            b.iter(|| {
                let mut tree = Tree::<String>::default();
                tree.act(actions.iter().cloned());
                tree
            });
        });
    }

    group.finish();
}

fn bench_act_insert_one_by_one(c: &mut Criterion) {
    let mut group = c.benchmark_group("act/insert_one_by_one");

    for n in [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000] {
        let values = make_values(n);
        let actions: Vec<Vec<Action<String>>> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                vec![Action::Insert {
                    party: "".to_string(),
                    version: i as u64,
                    value: v.clone(),
                }]
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(n), &actions, |b, actions| {
            b.iter(|| {
                let mut tree = Tree::<String>::default();
                for batch in actions {
                    tree.act(batch.iter().cloned());
                }
                tree
            });
        });
    }

    group.finish();
}

fn bench_act_insert_into_populated(c: &mut Criterion) {
    let mut group = c.benchmark_group("act/insert_batch_into_populated");

    for existing in [100, 1_000, 10_000] {
        let base_values = make_values(existing);
        let base_actions = make_inserts(&base_values);
        let mut base_tree = Tree::<String>::default();
        base_tree.act(base_actions);

        let new_values = make_values(100);
        let new_actions = make_inserts(&new_values);

        group.bench_with_input(
            BenchmarkId::new("100_into", existing),
            &(base_tree.clone(), new_actions),
            |b, (tree, actions)| {
                b.iter_batched(
                    || tree.clone(),
                    |mut tree| {
                        tree.act(actions.iter().cloned());
                        tree
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_baseline_hashmap(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline/hashmap_insert");

    for n in [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000] {
        let values = make_values(n);

        group.measurement_time(if n >= 100_000 {
            std::time::Duration::from_secs(15)
        } else {
            std::time::Duration::from_secs(5)
        });

        group.bench_with_input(BenchmarkId::from_parameter(n), &values, |b, values| {
            b.iter(|| {
                let mut map = HashMap::<[u8; 32], (u64, u64, Bytes)>::new();
                for (i, v) in values.iter().enumerate() {
                    let hash = *blake3::hash(v).as_bytes();
                    map.insert(hash, (0u64, i as u64, v.clone()));
                }
                map
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_act_insert_batch,
    bench_act_insert_one_by_one,
    bench_act_insert_into_populated,
    bench_baseline_hashmap,
);
criterion_main!(benches);
