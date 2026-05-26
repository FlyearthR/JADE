# JADE — Just A Deterministic Emulator

**Overview**

JADE is an user-space deterministic emulator designed to enable reproducible, time-deterministic execution of real protocol implementations. It interposes a small subset of libc calls (time, randomness, and blocking I/O) to advance a global, event-driven simulated clock while delegating most system functionality to the host libc and kernel.

**Key Features**
- **Deterministic Time**: Virtualized, event-driven clock that advances only on scheduled events.
- **Minimal Interposition**: Only time- and randomness-related libc calls are simulated; other behavior is delegated to the host.
- **Binary Compatibility**: Runs unmodified, dynamically linked executables via LD_PRELOAD-based interposition.
- **Ivy Integration**: Restores operational soundness to Ivy’s Network-centric Compositional Testing (NCT) by making timing deterministic and reproducible.
- **Extensible**: Simple model for adding support for additional libc functions.

**Quick Start**

- Build: The project contains Rust and C components. From the repository root you can typically build with:

	``make``

- Alternatively, build the Rust core directly:

	``cargo build --release``

- Tests and examples live under the `tests/` folder.

**Running Model-Based Tests with Ivy**

- Use JADE together with Ivy to run deterministic, timing-sensitive model-based tests. JADE virtualizes time (so solver latency does not interfere with deadlines) and schedules packet delivery to preserve reproducibility.

**Repository Layout**
- **`src/`**: Rust core and library code.
- **`c_src/`**: C helper code and the libc interposition layer.
- **`tests/`**: Test cases and tools used in the evaluation.
- **`XP/`**: To reproduce the graphs presented in the paper. The folder contains step-by-step instructions to reproduce the figures 3 and 4 from the paper.

**Contributing**
- To add support for a libc call, implement the simulated parts in the interposition layer and delegate the rest to the host libc. Follow the style in `c_src/`.

**Research**
- JADE is presented in the IFIP Networking 26 paper at https://dl.ifip.org/db/conf/networking/networking2026/1571262391.pdf.

