# How to Reproduce

Follow the steps below to reproduce this project from scratch.

## 1. Clone Asterinas and Check Out the Target Commit

```bash
git clone https://github.com/asterinas/asterinas.git
cd asterinas
git checkout f05e89b615c5dcb3f7c74accf24bdc23f96fcfc3
```

## 2. Apply the Patches

From the root of the Asterinas repository, apply all patches in this directory in order:

```bash
git am /path/to/artifacts/patches/*.patch
```

Replace `/path/to/` with the actual path to this repository on your machine.

## 3. Start the Development Container

Follow the instructions in Asterinas' `README.md` to launch the development container. Typically:

```bash
docker run -it --privileged \
    --network=host \
    -v /dev:/dev \
    -v $(pwd)/asterinas:/root/asterinas asterinas/asterinas:0.17.1-20260317
```

Refer to the Asterinas README for the exact image tag and any additional flags required.

## 4. Build Codex as a Static Binary

Inside the container, clone the Codex source code and build it using the provided script:

```bash
git clone https://github.com/openai/codex.git codex-src
bash tools/build_codex_musl.sh
```

If the build fails due to missing libraries, install them and re-run the script. The output is a statically linked `codex` binary suitable for running inside the Asterinas guest.

## 5. Build the NixOS Image

From the root of the Asterinas repository inside the container, build the NixOS guest image:

```bash
make nixos
```

## 6. Start the Asterinas Guest

Launch the Asterinas guest with the NixOS image:

```bash
make run_nixos
```

## 7. Launch Codex Inside the Guest

Wait for the guest to finish booting. Once the guest is ready, start the `codex` agent inside it:

```bash
codex
```
