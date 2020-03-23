Simplistic cache implementation, allowing basic operations: get, set, publish and subscribe.

# Benchmark comparison of various concurrent cache implementations from a single threaded caller

- r2d2-redis
- redis-async without pool with pipelining
- deadpool-redis async pool
- bb8-redis async pool
- redis-async with fixed set of connections with pipelining

*) redis-async handles reconnection logic under the hood, though while reconnecting there might be error responses.

# Conclusions based on below data:

- Redis-async provides fastest response times, though it might be less stable on response times, which might be ok for cache
- Most pools will start timing out over certain number of concurrent connections. It seems between 32 and 64, but will heavily depend on setup.


### r2d2-redis

```
r2d2-redis with pool of connections/r2d2-redis with pool of connections                                                                            
                        time:   [1.9925 ms 2.4382 ms 3.0482 ms]
                        thrpt:  [7.3328 MiB/s 9.1675 MiB/s 11.218 MiB/s]
Response times percentiles:
0.02% < 1.361ms
50.07% < 1.925ms
75.29% < 2.103ms
87.58% < 2.291ms
93.77% < 2.551ms
96.93% < 2.891ms
98.44% < 3.327ms
99.23% < 4.015ms
```


### ========== CONCUREENCY == 1 ============

```
redis-async with single connection/concurrency = 1                                                                            
                        time:   [1.3815 ms 1.5000 ms 1.6075 ms]
                        thrpt:  [13.905 MiB/s 14.901 MiB/s 16.180 MiB/s]
Response times percentiles:
0.02% < 0.905ms
50.04% < 1.526ms
75.03% < 1.761ms
87.52% < 1.974ms
93.77% < 2.215ms
96.90% < 2.509ms
98.45% < 2.911ms
99.24% < 3.609ms


deadpool-redis with pool of connections/concurrency = 1                                                                            
                        time:   [2.1201 ms 2.7085 ms 3.8693 ms]
                        thrpt:  [5.7769 MiB/s 8.2528 MiB/s 10.543 MiB/s]
Response times percentiles:
0.02% < 1.470ms
50.32% < 2.141ms
75.16% < 2.381ms
87.51% < 2.621ms
93.79% < 2.941ms
96.91% < 3.383ms
98.44% < 3.997ms
99.23% < 5.163ms


bb8-redis with pool of connections/concurrency = 1                                                                            
                        time:   [1.9171 ms 2.0188 ms 2.1572 ms]
                        thrpt:  [10.362 MiB/s 11.072 MiB/s 11.659 MiB/s]
Response times percentiles:
0.02% < 1.411ms
50.08% < 1.938ms
75.11% < 2.123ms
87.52% < 2.321ms
93.76% < 2.575ms
96.88% < 2.893ms
98.45% < 3.455ms
99.23% < 4.203ms


redis-async-pool with pool of connections/concurrency = 1                                                                            
                        time:   [1.2669 ms 1.3117 ms 1.3636 ms]
                        thrpt:  [16.391 MiB/s 17.040 MiB/s 17.643 MiB/s]
Response times percentiles:
0.01% < 0.865ms
50.00% < 1.246ms
75.16% < 1.407ms
87.51% < 1.608ms
93.75% < 1.925ms
96.88% < 2.285ms
98.44% < 2.847ms
99.23% < 3.755ms
```

### ========== CONCUREENCY == 4 ============

```
redis-async with single connection/concurrency = 4                                                                            
                        time:   [609.94 us 712.44 us 806.94 us]
                        thrpt:  [27.700 MiB/s 31.374 MiB/s 36.646 MiB/s]
Response times percentiles:
0.01% < 0.901ms
50.15% < 2.449ms
75.07% < 2.951ms
87.52% < 3.377ms
93.76% < 3.831ms
96.88% < 4.303ms
98.44% < 5.055ms
99.22% < 6.243ms


deadpool-redis with pool of connections/concurrency = 4                                                                            
                        time:   [812.88 us 1.0350 ms 1.2058 ms]
                        thrpt:  [18.537 MiB/s 21.596 MiB/s 27.498 MiB/s]
Response times percentiles:
0.01% < 1.628ms
50.01% < 3.601ms
75.04% < 4.347ms
87.53% < 5.087ms
93.76% < 5.927ms
96.90% < 7.255ms
98.45% < 9.295ms
99.22% < 13.919ms


bb8-redis with pool of connections/concurrency = 4                                                                            
                        time:   [863.69 us 920.65 us 994.01 us]
                        thrpt:  [22.487 MiB/s 24.279 MiB/s 25.880 MiB/s]
Response times percentiles:
0.01% < 1.532ms
50.11% < 3.523ms
75.00% < 4.219ms
87.54% < 4.867ms
93.76% < 5.515ms
96.88% < 6.375ms
98.47% < 7.635ms
99.22% < 9.183ms


redis-async-pool with pool of connections/concurrency = 4                                                                            
                        time:   [586.60 us 645.72 us 681.37 us]
                        thrpt:  [32.805 MiB/s 34.616 MiB/s 38.105 MiB/s]
Response times percentiles:
0.01% < 0.915ms
50.08% < 2.359ms
75.00% < 2.877ms
87.51% < 3.367ms
93.75% < 3.881ms
96.88% < 4.475ms
98.44% < 5.167ms
99.22% < 6.255ms
```

### ========== CONCUREENCY == 8 ============

```
redis-async with single connection/concurrency = 8                                                                            
                        time:   [569.16 us 641.81 us 729.39 us]
                        thrpt:  [30.645 MiB/s 34.827 MiB/s 39.273 MiB/s]
Response times percentiles:
0.01% < 1.457ms
50.05% < 4.639ms
75.02% < 5.587ms
87.52% < 6.411ms
93.76% < 7.171ms
96.89% < 7.923ms
98.44% < 8.815ms
99.23% < 9.839ms


deadpool-redis with pool of connections/concurrency = 8                                                                            
                        time:   [692.31 us 730.91 us 799.13 us]
                        thrpt:  [27.971 MiB/s 30.581 MiB/s 32.286 MiB/s]
Response times percentiles:
0.01% < 2.467ms
50.07% < 5.547ms
75.11% < 6.583ms
87.52% < 7.583ms
93.77% < 8.687ms
96.88% < 9.703ms
98.44% < 10.847ms
99.22% < 12.415ms


bb8-redis with pool of connections/concurrency = 8                                                                            
                        time:   [721.55 us 768.76 us 838.52 us]
                        thrpt:  [26.657 MiB/s 29.076 MiB/s 30.978 MiB/s]
Response times percentiles:
0.01% < 2.489ms
50.00% < 5.851ms
75.07% < 7.199ms
87.53% < 8.559ms
93.76% < 9.983ms
96.89% < 11.695ms
98.44% < 13.279ms
99.22% < 15.575ms


redis-async-pool with pool of connections/concurrency = 8                                                                            
                        time:   [526.41 us 555.16 us 592.88 us]
                        thrpt:  [37.701 MiB/s 40.263 MiB/s 42.461 MiB/s]
Response times percentiles:
0.01% < 0.930ms
50.07% < 4.085ms
75.00% < 5.179ms
87.52% < 6.183ms
93.76% < 7.247ms
96.88% < 8.367ms
98.44% < 9.631ms
99.22% < 10.887ms
```

### ========== CONCUREENCY == 16 ============

```
redis-async with single connection/concurrency = 16                                                                            
                        time:   [543.71 us 592.92 us 695.02 us]
                        thrpt:  [32.160 MiB/s 37.699 MiB/s 41.111 MiB/s]
Response times percentiles:
0.01% < 1.624ms
50.13% < 8.783ms
75.05% < 9.951ms
87.55% < 11.223ms
93.75% < 12.671ms
96.89% < 14.087ms
98.44% < 16.015ms
99.23% < 18.127ms


deadpool-redis with pool of connections/concurrency = 16                                                                            
                        time:   [732.79 us 803.18 us 849.94 us]
                        thrpt:  [26.299 MiB/s 27.830 MiB/s 30.503 MiB/s]
Response times percentiles:
0.01% < 2.933ms
50.07% < 12.471ms
75.02% < 14.887ms
87.52% < 17.343ms
93.75% < 19.343ms
96.88% < 21.551ms
98.44% < 23.919ms
99.23% < 26.271ms


bb8-redis with pool of connections/concurrency = 16                                                                           
                        time:   [416.50 us 472.84 us 525.85 us]
                        thrpt:  [42.507 MiB/s 47.272 MiB/s 53.666 MiB/s]
Response times percentiles:
0.01% < 1.453ms
50.04% < 6.935ms
75.01% < 8.431ms
87.51% < 9.759ms
93.76% < 11.079ms
96.88% < 12.615ms
98.44% < 14.111ms
99.22% < 15.943ms

redis-async-pool with pool of connections/concurrency = 16                                                                           
                        time:   [455.58 us 468.44 us 480.89 us]
                        thrpt:  [46.481 MiB/s 47.716 MiB/s 49.064 MiB/s]
Response times percentiles:
0.01% < 1.476ms
50.02% < 6.883ms
75.04% < 8.839ms
87.52% < 10.583ms
93.77% < 12.279ms
96.88% < 14.239ms
98.44% < 16.223ms
99.22% < 18.543ms
```

### ========== CONCUREENCY == 32 ============

```
redis-async with single connection/concurrency = 32                                                                            
                        time:   [514.83 us 551.57 us 601.63 us]
                        thrpt:  [37.153 MiB/s 40.524 MiB/s 43.417 MiB/s]
Response times percentiles:
0.01% < 1.676ms
50.02% < 17.375ms
75.02% < 19.439ms
87.52% < 21.951ms
93.76% < 25.343ms
96.89% < 28.415ms
98.44% < 32.799ms
99.23% < 37.663ms


deadpool-redis with pool of connections/concurrency = 32                                                                            
                        time:   [719.01 us 783.13 us 859.94 us]
                        thrpt:  [25.993 MiB/s 28.542 MiB/s 31.087 MiB/s]
Response times percentiles:
0.01% < 2.767ms
50.07% < 22.719ms
75.01% < 26.095ms
87.52% < 29.823ms
93.76% < 34.687ms
96.91% < 38.943ms
98.44% < 42.911ms
99.22% < 47.743ms


bb8-redis with pool of connections/concurrency = 32                                                                           
                        time:   [416.69 us 452.22 us 505.82 us]
                        thrpt:  [44.190 MiB/s 49.428 MiB/s 53.643 MiB/s]
Response times percentiles:
0.01% < 2.811ms
50.07% < 13.783ms
75.02% < 16.079ms
87.56% < 18.719ms
93.78% < 22.207ms
96.88% < 25.759ms
98.45% < 29.967ms
99.22% < 35.647ms


redis-async-pool with pool of connections/concurrency = 32                                                                           
                        time:   [405.80 us 413.18 us 420.10 us]
                        thrpt:  [53.206 MiB/s 54.098 MiB/s 55.082 MiB/s]
Response times percentiles:
0.01% < 1.690ms
50.08% < 12.575ms
75.03% < 16.479ms
87.57% < 19.775ms
93.76% < 22.863ms
96.88% < 26.319ms
98.45% < 30.127ms
99.22% < 35.295ms
```

### ========== CONCUREENCY == 64 ============

```
redis-async with single connection/concurrency = 64                                                                            
                        time:   [488.45 us 512.34 us 556.89 us]
                        thrpt:  [40.138 MiB/s 43.628 MiB/s 45.761 MiB/s]
Response times percentiles:
0.01% < 2.061ms
50.08% < 32.559ms
75.18% < 36.735ms
87.53% < 41.055ms
93.78% < 45.023ms
96.91% < 48.511ms
98.45% < 51.839ms
99.22% < 55.807ms


deadpool-redis with pool of connections/concurrency = 64                                                                            
                        time:   [737.47 us 779.72 us 816.17 us]
                        thrpt:  [27.387 MiB/s 28.667 MiB/s 30.309 MiB/s]
Response times percentiles:
0.01% < 2.515ms
50.12% < 46.495ms
75.02% < 52.959ms
87.52% < 59.743ms
93.75% < 66.175ms
96.88% < 72.191ms
98.44% < 78.719ms
99.23% < 82.367ms


bb8-redis with pool of connections/concurrency = 64                                                                           
                        time:   [441.23 us 493.19 us 536.45 us]
                        thrpt:  [41.667 MiB/s 45.322 MiB/s 50.659 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild
Response times percentiles:
0.01% < 3.033ms
50.03% < 28.527ms
75.07% < 31.919ms
87.50% < 35.551ms
93.77% < 40.863ms
96.89% < 47.743ms
98.44% < 54.975ms
99.22% < 60.799ms


redis-async-pool with pool of connections/concurrency = 64                                                                           
                        time:   [404.87 us 417.24 us 431.88 us]
                        thrpt:  [51.755 MiB/s 53.572 MiB/s 55.209 MiB/s]
Response times percentiles:
0.01% < 1.689ms
50.03% < 25.567ms
75.07% < 34.207ms
87.50% < 43.135ms
93.75% < 51.935ms
96.88% < 62.207ms
98.45% < 68.735ms
99.24% < 72.767ms
```
