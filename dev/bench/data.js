window.BENCHMARK_DATA = {
  "lastUpdate": 1767587148892,
  "repoUrl": "https://github.com/Na1w/infinitedsp",
  "entries": {
    "Rust Benchmark": [
      {
        "commit": {
          "author": {
            "email": "fredrikandersson@mac.com",
            "name": "Fredrik Andersson",
            "username": "Na1w"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "937c5ee143ebcb56b3cb7b657a2c543193905c75",
          "message": "Add automatic performance regression for the benchmarks... decorate the README a bit. (#4)",
          "timestamp": "2026-01-05T05:13:37+01:00",
          "tree_id": "653f162daa4139fdda279debc2c7eb026fd81e05",
          "url": "https://github.com/Na1w/infinitedsp/commit/937c5ee143ebcb56b3cb7b657a2c543193905c75"
        },
        "date": 1767587148510,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "dsp_benchmarks::oscillator::bench_oscillator_sine",
            "value": 40220,
            "unit": "instructions"
          },
          {
            "name": "dsp_benchmarks::oscillator::bench_oscillator_saw",
            "value": 21333,
            "unit": "instructions"
          },
          {
            "name": "dsp_benchmarks::oscillator::bench_oscillator_square",
            "value": 51906,
            "unit": "instructions"
          },
          {
            "name": "dsp_benchmarks::oscillator::bench_oscillator_noise",
            "value": 20775,
            "unit": "instructions"
          },
          {
            "name": "dsp_benchmarks::reverb::bench_reverb",
            "value": 313818,
            "unit": "instructions"
          }
        ]
      }
    ]
  }
}