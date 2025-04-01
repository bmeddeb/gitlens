"""
GitLens: A comprehensive read-only Git repository analysis library.

This library provides powerful tools for analyzing Git repositories,
including commit history, code ownership, file change frequency,
branch divergence, and other metrics.
"""

__version__ = "0.1.0"

try:
    from ._gitlens import *
except ImportError:
    from .api import *