import typing
import unittest

import anydoc


class FormatAliasTests(unittest.TestCase):
    def test_public_format_alias_matches_runtime_contract(self):
        self.assertEqual(
            typing.get_args(anydoc.Format),
            (
                "doc",
                "docx",
                "odt",
                "pdf",
                "ppt",
                "pptx",
                "rtf",
                "epub",
                "html",
                "mhtml",
                "xlsx",
                "ods",
                "odp",
                "csv",
            ),
        )


if __name__ == "__main__":
    unittest.main()
