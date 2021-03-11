# Copyright (c) Facebook, Inc. and its affiliates.
#
# This source code is licensed under the MIT license found in the
# LICENSE file in the root directory of this source tree.

from textwrap import dedent
from typing import Set

import libcst as cst
from libcst.metadata.name_provider import QualifiedNameProvider
from libcst.metadata.scope_provider import QualifiedName, QualifiedNameSource
from libcst.testing.utils import UnitTest


def get_qualified_names(module_str: str) -> Set[QualifiedName]:
    wrapper = cst.MetadataWrapper(cst.parse_module(dedent(module_str)))
    qnames = wrapper.resolve(QualifiedNameProvider)
    return set().union(*qnames.values())


class QualifiedNameProviderTest(UnitTest):
    def test_imports(self) -> None:
        qnames = get_qualified_names(
            """
            from a.b import c as d
            d
            """
        )
        self.assertEqual({"a.b.c"}, {qname.name for qname in qnames})
        for qname in qnames:
            self.assertEqual(qname.source, QualifiedNameSource.IMPORT, msg=f"{qname}")

    def test_builtins(self) -> None:
        qnames = get_qualified_names(
            """
            int(None)
            """
        )
        self.assertEqual(
            {"builtins.int", "builtins.None"}, {qname.name for qname in qnames}
        )
        for qname in qnames:
            self.assertEqual(qname.source, QualifiedNameSource.BUILTIN, msg=f"{qname}")

    def test_locals(self) -> None:
        qnames = get_qualified_names(
            """
            class X:
                a: "X"
            """
        )
        self.assertEqual({"X", "X.a"}, {qname.name for qname in qnames})
        for qname in qnames:
            self.assertEqual(qname.source, QualifiedNameSource.LOCAL, msg=f"{qname}")


class FullyQualifiedNameProviderTest(UnitTest):
    def test_builtins(self) -> None:
        qnames = get_fully_qualified_names(
            "test/module.py",
            """
            int(None)
            """,
        )
        qnames -= {QualifiedName(name="test.module", source=QualifiedNameSource.LOCAL)}
        self.assertEqual(
            {"builtins.int", "builtins.None"},
            {qname.name for qname in qnames},
        )
        for qname in qnames:
            self.assertEqual(qname.source, QualifiedNameSource.BUILTIN, msg=f"{qname}")

    def test_imports(self) -> None:
        qnames = get_fully_qualified_names(
            "some/test/module.py",
            """
            from a.b import c as d
            from . import rel
            from .lol import rel2
            from .. import thing as rel3
            d, rel, rel2, rel3
            """,
        )
        qnames -= {
            QualifiedName(name="some.test.module", source=QualifiedNameSource.LOCAL)
        }
        self.assertEqual(
            {"a.b.c", "some.test.rel", "some.test.lol.rel2", "some.thing"},
            {qname.name for qname in qnames},
        )
        for qname in qnames:
            self.assertEqual(qname.source, QualifiedNameSource.IMPORT, msg=f"{qname}")

    def test_locals(self) -> None:
        qnames = get_fully_qualified_names(
            "some/test/module.py",
            """
            class X:
                a: X
            """,
        )
        self.assertEqual(
            {"some.test.module", "some.test.module.X", "some.test.module.X.a"},
            {qname.name for qname in qnames},
        )
        for qname in qnames:
            self.assertEqual(qname.source, QualifiedNameSource.LOCAL, msg=f"{qname}")
