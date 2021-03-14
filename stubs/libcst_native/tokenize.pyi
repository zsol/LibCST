# Copyright (c) Facebook, Inc. and its affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

from libcst_native import token_type
from libcst_native import whitespace_state
from typing import Iterator, Optional, Tuple

class Token:
    def __new__(
        cls,
        type: token_type.TokenType,
        string: str,
        start_pos: Tuple[int, int],
        end_pos: Tuple[int, int],
        whitespace_before: whitespace_state.WhitespaceState,
        whitespace_after: whitespace_state.WhitespaceState,
        relative_indent: Optional[str],
    ) -> Token: ...

    type: token_type.TokenType
    string: str
    start_pos: Tuple[int, int]
    end_pos: Tuple[int, int]
    whitespace_before: whitespace_state.WhitespaceState
    whitespace_after: whitespace_state.WhitespaceState
    relative_indent: Optional[str]

def tokenize(text: str) -> Iterator[Token]: ...
