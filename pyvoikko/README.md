# PyVoikko

PyVoikko is an implementation of Voikko, a free and open source morphological analyzer for Finnish.
It aims to reimplement the Voikko library in pure Python, so that it can be used in environments in which native libraries like libvoikko cannot be installed.
It contains a pre-built version of the voikko-fi transducer and uses the [KFST](https://pypi.org/project/kfst/) library for lookups.

It is in early stages of development.
The only thing implemented right now is a parser for the analysis strings produced by the Voikko transducer.
As the analysis strings are quite complex, it is very likely that the parser does not yet parse all of them correctly.
Please report bugs if you find them!

Other features of the Voikko library, such as spell checking and tokenising, are not implemented yet.

## Installation

PyVoikko is available on PyPI and can be installed with pip:

```sh
pip install pyvoikko
```

## Usage

```py
import pyvoikko

print(pyvoikko.analyse("kissa"))
```

## License

PyVoikko is licensed under the GNU GPL version 3 or later. See the LICENSE file for details.