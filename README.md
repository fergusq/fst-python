# Pure-Python Finite State Technology

I often use FST-based tools like Omorfi and Voikko in my projects.
However, parts of them are written in C++ and require native libraries like HFST and libvoikko to be installed.
This makes it difficult to use them in environments in which installing native libraries is not possible.
For example, it is far from trivial to use Omorfi in a classroom setting in which the students need to install it on their own computers.
Also, many web hosting services that support Python don't support installing native libraries easily.

This project contains three parts:

- [KFST](https://github.com/fergusq/fst-python/tree/main/kfst) is a finite state transducer library written in pure Python. It replaces HFST in Omorfi and VVFST in Voikko.
- [PyOmorfi](https://github.com/fergusq/fst-python/tree/main/pyomorfi) contains the Python bindings of Omorfi modified so that they use KFST instead of HFST.
- [PyVoikko](https://github.com/fergusq/fst-python/tree/main/pyvoikko) contains a parser for the analysis strings produced by the Voikko transducer. It too uses KFST for lookups.

Both PyOmorfi and PyVoikko contain pre-built transducers. The idea is that you can just install them from PyPI and they will just work.

## Installation

KFST, PyOmorfi and PyVoikko are available on PyPI and can be installed with pip:

```
pip install kfst
pip install pyomorfi
pip install pyvoikko
```

## Usage

Please refer to the README files of the individual projects for usage instructions.

## Speed

KFST is written in pure Python and is therefore much slower than native libraries like HFST and libvoikko.
For example, on my computer, libvoikko takes about 0.02 ms to analyse a word, while PyVoikko takes about 1 ms.
However, this is still fast enough for many use cases.

## Contributing

This project is in very early stages of its development and there are likely many bugs.
Please open an issue if you find one!

If you want to contribute code, feel free to open pull requests.

## License

KFST is licensed under the GNU LGPL version 3 or later. PyOmorfi is licensed under GNU GPL version 3 but **not** later. PyVoikko is licensed under the GNU GPL version 3 or later. See the LICENSE files for details.