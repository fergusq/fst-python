# kfst-rs

kfst-rs is both the optional acceleration back end of [kfst](https://github.com/fergusq/fst-python) and a self-standing rust implementation of finite-state transducers mostly compatible with [HFST](https://hfst.github.io/). These two functions are liable to being split into multiple crates / packages in the future.

Able to load and execute [Voikko](https://voikko.puimula.org/) and [Omorfi](https://github.com/flammie/omorfi):
see [kfst](https://github.com/fergusq/fst-python) for transducers converted to a compatible format as well as Python bindings.

Supports the ATT format and its own KFST format.
To convert HFST (optimized lookup or otherwise) to ATT using HFST's tools, do:
```bash
hfst-fst2txt transducer.hfst -o transducer.att
```

# Using from Python

If you have kfst>=4.1 installed, simply do

```bash
pip install kfst-rs
```

to install kfst-rs. It should get automatically picked up by kfst and by extension pyvoikko and pyomorfi.

To check which implementation of kfst got loaded, look at the `BACKEND` property of kfst.

# Using from rust

Given the Voikko transducer in KFST or ATT format, one could create a simple analyzer like this:
```rust
use kfst_rs::{FSTState, FST};
use std::io::{self, Write};
// Read in transducer
# let pathtovoikko = "../pyvoikko/pyvoikko/voikko.kfst".to_string();
let fst = FST::from_kfst_file(pathtovoikko, true).unwrap();
// Alternatively, for ATT use FST::from_att_file
// Read in word to analyze
let mut buffer = String::new();
let stdin = io::stdin();
stdin.read_line(&mut buffer).unwrap();
buffer = buffer.trim().to_string();
// Do analysis proper
match fst.lookup(&buffer, FSTState::default(), true) {
    Ok(result) => {
        for (i, analysis) in result.into_iter().enumerate() {
            println!("Analysis {}: {} ({})", i+1, analysis.0, analysis.1)
        }
    },
    Err(err) => println!("No analysis: {:?}", err),
}
```
Given the input "lentokoneessa", this gives the following analysis:
```text
Analysis 1: [Lt][Xp]lentää[X]len[Ln][Xj]to[X]to[Sn][Ny][Bh][Bc][Ln][Xp]kone[X]konee[Sine][Ny]ssa (0)
```