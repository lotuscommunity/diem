Scripts that help generate/evaluate gas parameters for generic algebra move module.

## Quickstart guide
Ensure you are on a machine with the [required spec](https://diem.dev/nodes/validator-node/operator/node-requirements/).

Ensure you have python3 and the following dependencies.
```
pip3 install numpy matplotlib
```

Ensure you `cd` to the repo root.

Run the necessary benches.
```
cargo bench -p diem-crypto -- hash/SHA2-256
cargo bench -p diem-crypto -- ark_bls12_381
```

Compute `gas_per_ns` using `hash/SHA2-256` bench results.
```
scripts/algebra-gas/load_bench_datapoints.py --bench_path target/criterion/hash/SHA2-256
scripts/algebra-gas/fit_linear_model.py --dataset_path hash_SHA2_256.0-1025.json --plot
```
This will fit a curve `f(n)=kn+b`
that predicts the time (in nanoseconds) to evaluate SHA2-256 on an input of size `n`.
Value `k` and `b` should be printed.
```
{"b": 336.51096106242346, "k": 4.868293006038344}
```

Combined with the [pre-defined](https://github.com/aptos-labs/diem-core/blob/28df4c1f0ea0d6c6dc6b0460257aa9086e830d1a/diem-move/diem-gas/src/move_stdlib.rs#L17-L18) SHA2-256 gas formula (unscaled internal gas):`g(n)=50n+3000`,
it can be calculated that `gas_per_ns = 50/k`.

Second last, go to `scripts/algebra-gas/update_algebra_gas_params.py`
and update the value of the global variable `TARGET_GAS_VERSION` if necessary.
See the comments on them for detailed instructions.

Now you can (re-)generate all algebra module gas parameters with one command.
```
scripts/algebra-gas/update_algebra_gas_params.py --gas_per_ns <gas_per_ns>
```

`git diff` to see the diff!
