import typing
import unittest

import anydoc


class FormatAliasTests(unittest.TestCase):
    def test_public_format_alias_includes_html_and_mhtml(self):
        formats = typing.get_args(anydoc.Format)
        self.assertIn("html", formats)
        self.assertIn("mhtml", formats)


if __name__ == "__main__":
    unittest.main()
