# PyOmorfi

This is a pure-python implementation of Omorfi, a free and open source morphological analyzer for Finnish.

Most of the files have been copied from the [Omorfi repository](https://github.com/flammie/omorfi).
HFST has been replaced with KFST, a pure-python implementation of finite state transducers.

This package includes a pre-built transducer, so you don't need to build the transducer yourself.
For most part, this library should be a drop-in replacement for Omorfi.

## Installation

Pyomorfi is available on PyPI and can be installed with pip:

```sh
pip install pyomorfi
```

## Usage

PyOmorfi has the same API as Omorfi, so you can use it in the same way as Omorfi.
Please refer to the [Omorfi documentation](https://flammie.github.io/omorfi/) for more information.
Just import `pyomorfi.omorfi` instead of `omorfi`.

```py
from pyomorfi.omorfi import Omorfi

omorfi = Omorfi()
omorfi.load_analyser("omorfi.analyse.kfst")

analyses = omorfi.analyse("kissa")
```

There is also a function that loads the built-in transducers and returns an `Omorfi` object:

```py
from pyomorfi import load_omorfi

omorfi = load_omorfi()

analyses = omorfi.analyse("kissa")
```

## Development

This package is in very early stages of its development and there are probably many bugs.
Please report them if you find them!

If you want to contribute code, feel free to open pull requests.

## License

PyOmorfi is licensed under the GNU GPL version 3. See the LICENSE file for details.