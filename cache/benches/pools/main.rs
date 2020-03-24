pub mod cache_error;
mod redis_async_connection;
mod redis_async_connection_pool;
mod redis_bb8;
mod redis_deadpool;
mod redis_r2d2;
use criterion::Throughput;
use criterion::{criterion_main, Criterion};
use crossbeam_queue::SegQueue;
use futures::future::{join_all, FutureExt};
use hdrhistogram::Histogram;
use redis_async_connection::RedisAsyncConnection;
use redis_async_connection_pool::RedisAsyncConnectionPool;
use redis_bb8::RedisBB8;
use redis_deadpool::RedisDeadpool;
use redis_r2d2::RedisR2D2;
use std::sync::Arc;

const DATA: &'static str = include_str!("./events.json");
const KEY: &'static str = "bench:pools";

fn bench_sync_pool(c: &mut Criterion, mut pool: RedisR2D2, name: &str) {
    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::Bytes(DATA.len() as u64));
    // start benchmark loops
    let response_times = Arc::new(SegQueue::<u64>::new());
    let response_times_rec = response_times.clone();
    group.bench_function(name, move |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            // benchmark body
            let res: Vec<Option<String>> = (1..iters)
                .map(|_| {
                    let start = std::time::Instant::now();
                    let res = pool.get(KEY);
                    response_times_rec.push(start.elapsed().as_micros() as u64);
                    match res {
                        Ok(res) => res,
                        Err(e) => {
                            println!("{:?} with Error {:?}", start.elapsed(), e);
                            Some("".to_owned())
                        }
                    }
                })
                .collect();
            let elapsed = start.elapsed();
            let _size: usize = res.into_iter().fold(0, |s, i| s + i.unwrap_or("".to_owned()).len());
            //println!("received {} bytes", size);
            elapsed
        })
    });
    responses_report(response_times);
}

macro_rules! impl_bench_async_pool {
    ($name:ident, $P:ty) => {
        fn $name(c: &mut Criterion, rt: &mut actix_rt::SystemRunner, pool: $P, name: &str, limit: usize) {
            let mut group = c.benchmark_group(name);
            group.throughput(Throughput::Bytes(DATA.len() as u64));

            let response_times = Arc::new(SegQueue::<u64>::new());
            let response_times_rec = response_times.clone();

            let name = format!("concurrency = {}", limit);
            // start benchmark loops
            group.bench_function(name, move |b| {
                b.iter_custom(|iters| {
                    let response_times_rec = response_times_rec.clone();
                    let pools: Vec<_> = (1..iters).map(|_| pool.clone()).collect();
                    let concurrency = tokio::sync::Semaphore::new(limit);
                    let start = std::time::Instant::now();
                    // benchmark body
                    let res = rt.block_on(async move {
                        join_all(pools.into_iter().map(|mut pool| {
                            let response_times_rec = response_times_rec.clone();
                            concurrency.acquire().then(|guard| async move {
                                let start = std::time::Instant::now();
                                let res = pool.get(KEY).await;
                                drop(guard);
                                response_times_rec.push(start.elapsed().as_micros() as u64);
                                match res {
                                    Ok(res) => res,
                                    Err(e) => {
                                        println!("{:?} with Error {:?}", start.elapsed(), e);
                                        Some("".to_owned())
                                    }
                                }
                            })
                        }))
                        .await
                    });
                    let elapsed = start.elapsed();
                    let _size: usize = res
                        .into_iter()
                        .fold(0, |s, i| s + i.unwrap_or("".to_owned()).len());
                    //println!("received {} bytes", size);
                    elapsed
                })
            });
            responses_report(response_times);
        }
    };
}

fn responses_report(data: Arc<SegQueue<u64>>) {
    let mut histo = Histogram::<u64>::new_with_bounds(1, 1000 * 1000 * 10, 3).unwrap();
    while !data.is_empty() {
        histo += data.pop().unwrap() as u64;
    }
    println!("Response times percentiles:");
    for val in histo.iter_quantiles(1).take(8) {
        println!(
            "{:.2}% < {:.3}ms",
            val.percentile(),
            val.value_iterated_to() as f64 / 1000.0
        );
    }
}

impl_bench_async_pool!(bench_async_conn, RedisAsyncConnection);
impl_bench_async_pool!(bench_deadpool, RedisDeadpool);
impl_bench_async_pool!(bench_bb8, RedisBB8);
impl_bench_async_pool!(bench_async_conn_pool, RedisAsyncConnectionPool);

const URL: &'static str = "redis://127.0.0.1/";
const TIMEOUT: u64 = 5_000;

pub fn service_benches() {
    let mut criterion: ::criterion::Criterion<_> = ::criterion::Criterion::default()
        .configure_from_args()
        .sample_size(10)
        .noise_threshold(0.1);

    let mut rt = actix_rt::System::new("test");

    let mut sync_conn = RedisR2D2::create_connection_pool(URL, TIMEOUT, TIMEOUT, TIMEOUT);
    let async_conn = rt
        .block_on(RedisAsyncConnection::create_connection(URL, TIMEOUT, TIMEOUT))
        .expect("failed to create async pool");
    let redis_deadpool =
        RedisDeadpool::create_connection_pool(URL, TIMEOUT, TIMEOUT, TIMEOUT, None).expect("failed to create deadpool");
    let redis_bb8 = rt.block_on(RedisBB8::create_connection_pool(URL, TIMEOUT, TIMEOUT, TIMEOUT));
    let async_conn_pool = rt
        .block_on(RedisAsyncConnectionPool::create_connection_pool(
            URL, TIMEOUT, TIMEOUT, None,
        ))
        .expect("failed to create async pool");

    sync_conn.add(KEY, DATA, Some(1_000_000));

    bench_sync_pool(&mut criterion, sync_conn, "r2d2-redis with pool of connections");

    for concurrency in &[1, 4, 8, 16, 32, 64] {
        println!("========== CONCUREENCY == {} ============", concurrency);
        bench_async_conn(
            &mut criterion,
            &mut rt,
            async_conn.clone(),
            "redis-async with single connection",
            *concurrency,
        );
        bench_deadpool(
            &mut criterion,
            &mut rt,
            redis_deadpool.clone(),
            "deadpool-redis with pool of connections",
            *concurrency,
        );
        bench_bb8(
            &mut criterion,
            &mut rt,
            redis_bb8.clone(),
            "bb8-redis with pool of connections",
            *concurrency,
        );
        bench_async_conn_pool(
            &mut criterion,
            &mut rt,
            async_conn_pool.clone(),
            "redis-async-pool with pool of connections",
            *concurrency,
        );
    }
}

criterion_main!(service_benches);
