# Pure-Python Finite State Technology

I often use FST-based tools like Omorfi and Voikko in my projects.
However, parts of them are written in C++ and require native libraries like HFST and libvoikko to be installed.
This makes it difficult to use them in environments in which installing native libraries is not possible.
For example, it is far from trivial to use Omorfi in a classroom setting in which the students need to install it on their own computers.
Also, many web hosting services that support Python don't support installing native libraries easily.

This project contains four parts:

- [KFST](https://github.com/fergusq/fst-python/tree/main/kfst) is a finite state transducer library written in pure Python. It replaces HFST in Omorfi and VVFST in Voikko.
- [PyOmorfi](https://github.com/fergusq/fst-python/tree/main/pyomorfi) contains the Python bindings of Omorfi modified so that they use KFST instead of HFST.
- [PyVoikko](https://github.com/fergusq/fst-python/tree/main/pyvoikko) contains a parser for the analysis strings produced by the Voikko transducer. It too uses KFST for lookups.
- [kfst\_rs](https://github.com/fergusq/fst-python/tree/main/kfst_rs) contains a an optional rust-accelerated back-end for KFST. This is both to improve speed and have a rust api: it can be compiled natively as a crate as well. It is designed to compile to python abi3 such that python updates don't require a recompilation (though seemingly the free-threaded Python project may complicate matters).

Both PyOmorfi and PyVoikko contain pre-built transducers. The idea is that you can just install them from PyPI and they will just work. Then, if you need the speed of the native component, installing should redirect internal calls to it. (Use kfst.BACKEND to check whether kfst-rs (value "kfst_rs") or the python implementation (value "kfst_py") is loaded.

## Installation

KFST, PyOmorfi and PyVoikko are available on PyPI and can be installed with pip:

```
pip install kfst
pip install pyomorfi
pip install pyvoikko
```

kfst\_rs is available as package `kfst-rs` on pypi pre-built for a number of platforms. As such, it should be installable with a simple

```
pip install kfst-rs
```

kfst-rs is also completely optional: every feature of kfst should work without it (if that isn't the case, that's a bug we want to know of). However, it can be around 4x faster in certain use-cases (generally those where the highest level interfaces are used).

We mostly test on the platforms we personally use, which should be Linux and macOS. If something else is broken, do ping us and we'll see if we can fix it. If your platform does not have a pre-built binary (let us know and we might make that happen too), the simplest setup assuming a python environment with maturin available as well as a rust toolchain is to `pip install <path/to/this/repo/kfst-rs>` which should build it automatically. The Dockerfile and bash scripts in the kfst-rs directory automate building wheels for the platforms we have pre-built wheels for; you should probably not need them if you just want to get it to work on your computer.

## Usage

Please refer to the README files of the individual projects for usage instructions.

## Speed

KFST is written in pure Python and is therefore much slower than native libraries like HFST and libvoikko.
For example, on my computer, libvoikko takes about 0.02 ms to analyse a word, while PyVoikko takes about 1 ms.
However, this is still fast enough for many use cases.

If speed is needed, have kfst-rs installed. It should be automatically detected and speed computation up.

## Contributing

This project is in very early stages of its development and there are likely many bugs.
Please open an issue if you find one!

If you want to contribute code, feel free to open pull requests.

## License

KFST is licensed under the GNU LGPL version 3 or later. PyOmorfi is licensed under GNU GPL version 3 but **not** later. PyVoikko is licensed under the GNU GPL version 3 or later. See the LICENSE files for details. kfst\_rs is distributed under GNU LGPL version 3 or later and the compiled rust component of it has dependencies under the following licenses:

```
MIT: lzma-rs, nom
MPL-2.0+: bitmaps, im, sized-chunks
Unlicense / MIT: byteorder, memchr
MIT / Apache-2.0: crc, crc-catalog, equivalent, hashbrown, indexmap, proc-macro2, quote, rand_core, rand_xoshiro, readonly, syn, typenum
(MIT / Apache-2.0) AND Unicode-3.0: unicode-ident
```
