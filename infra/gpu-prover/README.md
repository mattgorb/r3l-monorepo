#### In EC2 instance run the following:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install SP1 toolchain
curl -L https://sp1.succinct.xyz | bash
source $HOME/.sp1/bin/env
sp1up

# Verify CUDA (should show GPU info)
nvidia-smi
```

#### Locally:
```
rsync -avz --exclude target --exclude node_modules \
  ~/Desktop/projects/r3l-provenance/services/ \
  ec2-user@54.226.193.135:~/r3l-provenance/services/

```

#### Build verifier and prover:

```bash
cd ~/r3l-provenance/services/verifier && cargo build --release
cd ~/r3l-provenance/services/prover/script && cargo build --release
```

#### Run verifier:

```bash
cd ~/r3l-provenance/services/prover/script
TRUST_DIR=~/r3l-provenance/data/trust ~/r3l-provenance/services/verifier/target/release/verifier ~/r3l-provenance/data/samples/chatgpt.png > verify.json
```

#### Run prover with GPU:

```bash
cd ~/r3l-provenance/services/prover/script
SP1_PROVER=cuda ../target/release/prove \
  --file verify.json \
  --media ~/r3l-provenance/data/samples/chatgpt.png
```

**Note:** Deep Learning AMI has CUDA pre-installed. SP1 will use GPU automatically when `SP1_PROVER=cuda` is set.
