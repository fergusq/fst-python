# This file is part of KFST.
#
# (c) 2023 Iikka Hauhio <iikka.hauhio@helsinki.fi>
#
# KFST is free software: you can redistribute it and/or modify it under the
# terms of the GNU Lesser General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version.
#
# KFST is distributed in the hope that it will be useful, but WITHOUT ANY
# WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
# FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License for more
# details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with KFST. If not, see <https://www.gnu.org/licenses/>.

# Prefer kfst_rs, otherwise import from own implementation

BACKEND: str
"""
kfst has two potential back-ends: kfst_rs (written in rust) and kfst's built-in python implementation
"""

from typing import TYPE_CHECKING

try:
    from kfst_rs import FST as FST # type: ignore
    from kfst_rs import TokenizationException as TokenizationException # type: ignore
    from kfst_rs import transducer as transducer # type: ignore
    from kfst_rs import symbols as symbols # type: ignore
    BACKEND = "kfst_rs"
except ImportError:
    BACKEND = "kfst"

if TYPE_CHECKING or BACKEND == "kfst":
    from kfst_py.transducer import FST as FST
    from kfst_py.transducer import TokenizationException as TokenizationException
    from kfst_py import symbols as symbols
    from kfst_py import transducer as transducer
    BACKEND = "kfst"