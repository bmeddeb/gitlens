"""
Python API fallback implementation for GitLens.

This module provides a pure Python implementation of the GitLens API
that can be used when the compiled extension is not available.
It uses Git commands directly via subprocess.
"""

import os
import subprocess
import re
import json
from dataclasses import dataclass
from typing import List, Dict, Optional, Union, Tuple
from datetime import datetime
import tempfile

@dataclass
class CommitInfo:
    """Information about a Git commit."""
    hash: str
    short_hash: str
    author_name: str
    author_email: str
    timestamp: int
    message: str
    parents: List[str]

    @property
    def date(self) -> datetime:
        """Return the commit date as a datetime object."""
        return datetime.fromtimestamp(self.timestamp)


@dataclass
class BranchInfo:
    """Information about a Git branch."""
    name: str
    commit: str
    is_head: bool
    upstream: Optional[str] = None


@dataclass
class StatusEntry:
    """Information about a file in the repository status."""
    path: str
    status: str
    original_path: Optional[str] = None


@dataclass
class StatusInfo:
    """Information about the repository status."""
    branch: Optional[str]
    files: List[StatusEntry]
    is_clean: bool
    merging: bool = False
    rebasing: bool = False
    cherry_picking: bool = False


@dataclass
class AuthorStats:
    """Statistics about an author's contributions."""
    commits: int
    added_lines: int
    removed_lines: int
    files_changed: int
    first_commit: int
    last_commit: int


@dataclass
class ContributionStats:
    """Statistics about contributions to a repository."""
    by_author: Dict[str, AuthorStats]
    total_commits: int
    total_authors: int
    total_added: int
    total_removed: int
    total_files_changed: int


@dataclass
class FileChangeFrequency:
    """Information about how frequently a file changes."""
    path: str
    change_count: int
    last_modified: int
    authors: List[str]
    primary_owner: Optional[str] = None


@dataclass
class BranchDivergence:
    """Information about how branches have diverged."""
    source: str
    target: str
    merge_base: str
    ahead_count: int
    behind_count: int
    differing_files: int


class Repository:
    """Represents a Git repository."""

    def __init__(self, path: str):
        """Initialize a repository instance pointing to the given path."""
        self.path = os.path.abspath(path)

    @classmethod
    def open(cls, path: str) -> 'Repository':
        """Open an existing Git repository."""
        repo = cls(path)
        # Check if it's a valid Git repository
        try:
            result = repo._run_git_command(["rev-parse", "--is-inside-work-tree"])
            if result.strip() != "true":
                raise ValueError(f"Not a Git repository: {path}")
        except subprocess.CalledProcessError:
            raise ValueError(f"Not a Git repository: {path}")

        return repo

    @classmethod
    def clone(cls, url: str, path: str, **options) -> 'Repository':
        """Clone a Git repository."""
        args = ["git", "clone"]

        # Apply clone options
        if options.get("shallow", False):
            args.extend(["--depth", "1"])

        if options.get("no_blob_content", False):
            args.append("--filter=blob:none")

        if options.get("single_branch") is not None:
            args.extend(["--single-branch", "--branch", options["single_branch"]])

        if options.get("bare", False):
            args.append("--bare")

        if options.get("mirror", False):
            args.append("--mirror")

        args.extend([url, path])

        try:
            subprocess.run(args, check=True, capture_output=True, text=True)
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Failed to clone repository: {e.stderr}")

        return cls.open(path)

    def _run_git_command(self, args: List[str]) -> str:
        """Run a Git command and return its output."""
        cmd = ["git"]
        cmd.extend(args)
        try:
            result = subprocess.run(
                cmd,
                cwd=self.path,
                check=True,
                capture_output=True,
                text=True
            )
            return result.stdout
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Git command failed: {e.stderr}")

    def list_branches(self) -> List[str]:
        """List all branches in the repository."""
        output = self._run_git_command(["branch", "--list", "--format=%(refname:short)"])
        return [line.strip() for line in output.splitlines() if line.strip()]

    def list_branches_info(self) -> List[BranchInfo]:
        """List branches with detailed information."""
        output = self._run_git_command([
            "branch", "--list", "-v",
            "--format=%(refname:short) %(objectname) %(HEAD) %(upstream:short)"
        ])

        branches = []
        for line in output.splitlines():
            if not line.strip():
                continue

            parts = line.split()
            if len(parts) >= 3:
                name = parts[0]
                commit = parts[1]
                is_head = parts[2] == "*"
                upstream = parts[3] if len(parts) >= 4 else None

                branches.append(BranchInfo(name, commit, is_head, upstream))

        return branches

    def list_tracked(self) -> List[str]:
        """List all tracked files in the repository."""
        output = self._run_git_command(["ls-files"])
        return [line.strip() for line in output.splitlines() if line.strip()]

    def get_hash(self, short: bool = False) -> str:
        """Get the current HEAD commit hash."""
        args = ["rev-parse"]
        if short:
            args.append("--short")
        args.append("HEAD")

        return self._run_git_command(args).strip()

    def get_commit(self, commit_ref: Optional[str] = None) -> CommitInfo:
        """Get information about a commit."""
        args = ["show", "--no-patch"]
        format_str = "%H%n%h%n%an%n%ae%n%at%n%P%n%s"
        args.append(f"--format={format_str}")

        if commit_ref is not None:
            args.append(commit_ref)

        output = self._run_git_command(args)
        lines = output.splitlines()

        if len(lines) < 7:
            raise ValueError(f"Failed to parse commit information")

        return CommitInfo(
            hash=lines[0],
            short_hash=lines[1],
            author_name=lines[2],
            author_email=lines[3],
            timestamp=int(lines[4]),
            parents=lines[5].split() if lines[5] else [],
            message=lines[6]
        )

    def status(self) -> StatusInfo:
        """Get the current status of the repository."""
        output = self._run_git_command(["status", "--porcelain=v2", "--branch"])

        branch_name = None
        files = []

        # Check special repository states
        git_dir = os.path.join(self.path, ".git")
        merging = os.path.exists(os.path.join(git_dir, "MERGE_HEAD"))
        rebasing = (
                os.path.exists(os.path.join(git_dir, "rebase-apply")) or
                os.path.exists(os.path.join(git_dir, "rebase-merge"))
        )
        cherry_picking = os.path.exists(os.path.join(git_dir, "CHERRY_PICK_HEAD"))

        for line in output.splitlines():
            if line.startswith("# branch.head "):
                branch_name = line.replace("# branch.head ", "").strip()
            elif line.startswith(("1 ", "2 ", "u ")):
                # Parse status codes and file paths
                parts = line.split()
                if len(parts) >= 2:
                    status_code = parts[1]
                    path_parts = line.split('\t')

                    if len(path_parts) > 1:
                        path = path_parts[1].strip()
                        original_path = path_parts[2].strip() if len(path_parts) > 2 else None

                        if status_code[0] == 'M':
                            status = "Modified"
                        elif status_code[0] == 'A':
                            status = "Added"
                        elif status_code[0] == 'D':
                            status = "Deleted"
                        elif status_code[0] == 'R':
                            status = "Renamed"
                        elif status_code[0] == 'C':
                            status = "Copied"
                        elif status_code[0] == 'U':
                            status = "Unmerged"
                        else:
                            status = "Unknown"

                        files.append(StatusEntry(path, status, original_path))
            elif line.startswith("? "):
                path = line[2:].strip()
                files.append(StatusEntry(path, "Untracked"))

        # Determine if working directory is clean
        is_clean = all(entry.status in ["Unmodified", "Ignored"] for entry in files)

        return StatusInfo(
            branch=branch_name,
            files=files,
            is_clean=is_clean,
            merging=merging,
            rebasing=rebasing,
            cherry_picking=cherry_picking
        )

    def get_commit_history(
            self,
            limit: Optional[int] = None,
            skip: Optional[int] = None,
            branch: Optional[str] = None
    ) -> List[CommitInfo]:
        """Get the commit history of the repository."""
        args = ["log"]

        format_str = "%H%n%h%n%an%n%ae%n%at%n%P%n%s%n--GitLensSeparator--"
        args.append(f"--format={format_str}")

        if limit is not None:
            args.extend(["--max-count", str(limit)])

        if skip is not None:
            args.extend(["--skip", str(skip)])

        if branch is not None:
            args.append(branch)

        output = self._run_git_command(args)
        commit_blocks = output.split("--GitLensSeparator--\n")

        commits = []
        for block in commit_blocks:
            if not block.strip():
                continue

            lines = block.splitlines()
            if len(lines) < 7:
                continue

            commits.append(CommitInfo(
                hash=lines[0],
                short_hash=lines[1],
                author_name=lines[2],
                author_email=lines[3],
                timestamp=int(lines[4]),
                parents=lines[5].split() if lines[5] else [],
                message=lines[6]
            ))

        return commits

    def analyze_contributions(self, limit: Optional[int] = None) -> ContributionStats:
        """Analyze contributor statistics for the repository."""
        commits = self.get_commit_history(limit=limit)

        author_stats = {}
        total_added = 0
        total_removed = 0
        total_files_changed = 0

        for commit in commits:
            # Get parent commit for diff
            parent_ref = commit.parents[0] if commit.parents else "4b825dc642cb6eb9a060e54bf8d69288fbee4904"

            # Get diff stats
            diff_output = self._run_git_command([
                "diff", "--numstat", parent_ref, commit.hash
            ])

            added_lines = 0
            removed_lines = 0
            files_changed = 0

            for line in diff_output.splitlines():
                if not line.strip():
                    continue

                parts = line.split()
                if len(parts) >= 3:
                    if parts[0] != "-":  # Skip binary files
                        added_lines += int(parts[0])
                    if parts[1] != "-":  # Skip binary files
                        removed_lines += int(parts[1])
                    files_changed += 1

            # Update author stats
            if commit.author_name not in author_stats:
                author_stats[commit.author_name] = AuthorStats(
                    commits=0,
                    added_lines=0,
                    removed_lines=0,
                    files_changed=0,
                    first_commit=commit.timestamp,
                    last_commit=commit.timestamp
                )

            stats = author_stats[commit.author_name]
            stats.commits += 1
            stats.added_lines += added_lines
            stats.removed_lines += removed_lines
            stats.files_changed += files_changed
            stats.first_commit = min(stats.first_commit, commit.timestamp)
            stats.last_commit = max(stats.last_commit, commit.timestamp)

            # Update global stats
            total_added += added_lines
            total_removed += removed_lines
            total_files_changed += files_changed

        return ContributionStats(
            by_author=author_stats,
            total_commits=len(commits),
            total_authors=len(author_stats),
            total_added=total_added,
            total_removed=total_removed,
            total_files_changed=total_files_changed
        )

    def analyze_change_frequency(self, path_filter: Optional[str] = None) -> List[FileChangeFrequency]:
        """Analyze file change frequency in the repository."""
        args = ["log", "--name-only", "--pretty=format:%an%n%at"]

        if path_filter is not None:
            args.extend(["--", path_filter])

        output = self._run_git_command(args)

        change_counts = {}
        last_modified = {}
        file_authors = {}

        current_author = None
        current_timestamp = None

        for line in output.splitlines():
            if not line.strip():
                current_author = None
                current_timestamp = None
            elif current_author is None:
                current_author = line
            elif current_timestamp is None:
                current_timestamp = int(line)
            else:
                # This is a file path
                path = line

                # Update change count
                if path not in change_counts:
                    change_counts[path] = 0
                change_counts[path] += 1

                # Update last modified
                if path not in last_modified or current_timestamp > last_modified[path]:
                    last_modified[path] = current_timestamp

                # Update author counts
                if path not in file_authors:
                    file_authors[path] = {}
                if current_author not in file_authors[path]:
                    file_authors[path][current_author] = 0
                file_authors[path][current_author] += 1

        # Create result list
        result = []
        for path, count in change_counts.items():
            authors = list(file_authors.get(path, {}).keys())

            # Find primary owner
            primary_owner = None
            max_count = 0
            for author, author_count in file_authors.get(path, {}).items():
                if author_count > max_count:
                    max_count = author_count
                    primary_owner = author

            result.append(FileChangeFrequency(
                path=path,
                change_count=count,
                last_modified=last_modified.get(path, 0),
                authors=authors,
                primary_owner=primary_owner
            ))

        # Sort by change count (descending)
        result.sort(key=lambda x: x.change_count, reverse=True)

        return result

    def calculate_branch_divergence(self, source: str, target: str) -> BranchDivergence:
        """Calculate divergence metrics between two branches."""
        # Find common ancestor
        merge_base = self._run_git_command(["merge-base", source, target]).strip()

        # Count commits ahead (source not in target)
        ahead_output = self._run_git_command([
            "rev-list", "--count", f"{target}..{source}"
        ])
        ahead_count = int(ahead_output.strip())

        # Count commits behind (target not in source)
        behind_output = self._run_git_command([
            "rev-list", "--count", f"{source}..{target}"
        ])
        behind_count = int(behind_output.strip())

        # Count differing files
        diff_output = self._run_git_command([
            "diff", "--name-only", source, target
        ])
        differing_files = len([line for line in diff_output.splitlines() if line.strip()])

        return BranchDivergence(
            source=source,
            target=target,
            merge_base=merge_base,
            ahead_count=ahead_count,
            behind_count=behind_count,
            differing_files=differing_files
        )

    def find_merge_base(self, commit1: str, commit2: str) -> str:
        """Find the common ancestor of two commits."""
        return self._run_git_command(["merge-base", commit1, commit2]).strip()

    def blame(self, file_path: str) -> List[dict]:
        """Get blame information for a file."""
        output = self._run_git_command(["blame", "--porcelain", file_path])

        blame_lines = []
        current_commit = None
        current_author = None
        current_timestamp = None
        current_line_no = None
        current_content = None

        for line in output.splitlines():
            if line.startswith('\t'):
                # This is a content line
                if current_commit is not None:
                    blame_lines.append({
                        'commit': current_commit,
                        'author': current_author,
                        'timestamp': current_timestamp,
                        'line_no': current_line_no,
                        'content': line[1:],  # Remove the tab character
                    })
            elif line.startswith('author '):
                current_author = line[7:]
            elif line.startswith('author-time '):
                current_timestamp = int(line[12:])
            elif line.startswith('original-line '):
                current_line_no = int(line[14:])
            elif len(line) >= 40 and all(c in '0123456789abcdef' for c in line[:40]):
                # This is likely a commit hash
                current_commit = line[:40]

        return blame_lines

    def analyze_code_ownership(self) -> dict:
        """Analyze code ownership in the repository."""
        files = self.list_tracked()

        file_ownership = {}
        dir_ownership = {}

        for file_path in files:
            try:
                blame_data = self.blame(file_path)

                # Count lines by author
                author_counts = {}
                for line in blame_data:
                    author = line['author']
                    if author not in author_counts:
                        author_counts[author] = 0
                    author_counts[author] += 1

                # Store file ownership
                file_ownership[file_path] = author_counts

                # Update directory ownership
                dir_path = os.path.dirname(file_path)
                if dir_path not in dir_ownership:
                    dir_ownership[dir_path] = {}

                for author, count in author_counts.items():
                    if author not in dir_ownership[dir_path]:
                        dir_ownership[dir_path][author] = 0
                    dir_ownership[dir_path][author] += count

            except Exception as e:
                # Skip files that can't be blamed
                print(f"Warning: Could not analyze {file_path}: {e}")
                continue

        return {
            'files': file_ownership,
            'directories': dir_ownership
        }


class Analytics:
    """Analytics for Git repositories."""

    @staticmethod
    def analyze_commit_history_timeline(repo: Repository, period: str = 'day', options: dict = None) -> Dict[str, int]:
        """Analyze the commit activity distribution over time."""
        if options is None:
            options = {}

        args = ["log", "--pretty=format:%at"]

        if not options.get('include_merges', True):
            args.append("--no-merges")

        if options.get('author_filter'):
            args.extend(["--author", options['author_filter']])

        if options.get('since_date'):
            args.extend(["--since", f"@{options['since_date']}"])

        if options.get('until_date'):
            args.extend(["--until", f"@{options['until_date']}"])

        if options.get('max_commits'):
            args.extend(["--max-count", str(options['max_commits'])])

        if options.get('skip_commits'):
            args.extend(["--skip", str(options['skip_commits'])])

        if options.get('path_filter'):
            args.extend(["--", options['path_filter']])

        output = repo._run_git_command(args)
        timestamps = [int(line.strip()) for line in output.splitlines() if line.strip()]

        timeline = {}
        for timestamp in timestamps:
            # Format timestamp according to period
            key = Analytics._format_time_period(timestamp, period)
            if key not in timeline:
                timeline[key] = 0
            timeline[key] += 1

        return timeline

    @staticmethod
    def _format_time_period(timestamp: int, period: str) -> str:
        """Format a timestamp according to the specified time period."""
        dt = datetime.fromtimestamp(timestamp)

        if period == 'hour':
            return dt.strftime('%Y-%m-%d %H:00')
        elif period == 'day':
            return dt.strftime('%Y-%m-%d')
        elif period == 'week':
            # Calculate ISO week number
            week_number = dt.isocalendar()[1]
            return f"{dt.year}-W{week_number:02d}"
        elif period == 'month':
            return dt.strftime('%Y-%m')
        elif period == 'year':
            return dt.strftime('%Y')
        else:
            raise ValueError(f"Unknown period: {period}")

    @staticmethod
    def analyze_file_evolution(repo: Repository, file_path: str, options: dict = None) -> List[dict]:
        """Analyze the evolution of a file over time."""
        if options is None:
            options = {}

        args = ["log", "--follow", "--patch", "--pretty=format:%H%n%an%n%ae%n%at%n%s"]

        if options.get('max_commits'):
            args.extend(["--max-count", str(options['max_commits'])])

        args.extend(["--", file_path])

        output = repo._run_git_command(args)

        # Parse output
        evolution = []
        current_commit = None
        current_author = None
        current_email = None
        current_timestamp = None
        current_message = None
        current_diff = []
        parsing_diff = False

        for line in output.splitlines():
            if current_commit is None:
                current_commit = line
            elif current_author is None:
                current_author = line
            elif current_email is None:
                current_email = line
            elif current_timestamp is None:
                current_timestamp = int(line)
            elif current_message is None:
                current_message = line
                parsing_diff = True
            elif parsing_diff:
                if line.startswith('diff --git ') and current_diff:
                    # Process previous diff
                    lines_added = sum(1 for l in current_diff if l.startswith('+') and not l.startswith('+++'))
                    lines_removed = sum(1 for l in current_diff if l.startswith('-') and not l.startswith('---'))

                    evolution.append({
                        'commit': current_commit,
                        'author': current_author,
                        'email': current_email,
                        'timestamp': current_timestamp,
                        'message': current_message,
                        'lines_added': lines_added,
                        'lines_removed': lines_removed,
                    })

                    current_commit = None
                    current_author = None
                    current_email = None
                    current_timestamp = None
                    current_message = None
                    current_diff = []
                    parsing_diff = False

                    # Start the next commit
                    current_commit = line

                # Add line to current diff
                current_diff.append(line)

        # Process the last diff
        if current_commit is not None and current_diff:
            lines_added = sum(1 for l in current_diff if l.startswith('+') and not l.startswith('+++'))
            lines_removed = sum(1 for l in current_diff if l.startswith('-') and not l.startswith('---'))

            evolution.append({
                'commit': current_commit,
                'author': current_author,
                'email': current_email,
                'timestamp': current_timestamp,
                'message': current_message,
                'lines_added': lines_added,
                'lines_removed': lines_removed,
            })

        return evolution

    @staticmethod
    def analyze_hotspots(repo: Repository, options: dict = None) -> List[dict]:
        """Identify code hotspots in the repository."""
        if options is None:
            options = {}

        # Get change frequency data
        frequency_data = repo.analyze_change_frequency()

        # Get tracked files
        tracked_files = repo.list_tracked()

        # Filter files
        source_extensions = ['.rs', '.py', '.java', '.js', '.ts', '.c', '.cpp', '.h', '.go', '.rb', '.php', '.cs']

        def is_source_file(path):
            ext = os.path.splitext(path)[1].lower()
            return ext in source_extensions and '/node_modules/' not in path and '/vendor/' not in path

        source_files = [f for f in tracked_files if is_source_file(f)]

        # Calculate complexity and hotspot factor
        hotspots = []
        for file_path in source_files:
            # Find file in frequency data
            frequency_entry = next((f for f in frequency_data if f.path == file_path), None)
            if frequency_entry is None:
                continue

            churn_factor = frequency_entry.change_count

            # Simple complexity heuristic based on file size and line count
            try:
                content = repo._run_git_command(["show", f"HEAD:{file_path}"])
                line_count = len(content.splitlines())
                complexity = line_count
            except:
                complexity = 0

            # Calculate hotspot factor
            hotspot_factor = churn_factor * (complexity ** 0.5) / 1000.0

            if hotspot_factor > 0:
                hotspots.append({
                    'file_path': file_path,
                    'churn_factor': churn_factor,
                    'complexity': complexity,
                    'hotspot_factor': hotspot_factor,
                })

        # Sort by hotspot factor (descending)
        hotspots.sort(key=lambda x: x['hotspot_factor'], reverse=True)

        return hotspots

    @staticmethod
    def analyze_knowledge_map(repo: Repository, options: dict = None) -> Dict[str, dict]:
        """Analyze the knowledge map of the repository."""
        if options is None:
            options = {}

        # Get code ownership data
        ownership = repo.analyze_code_ownership()

        # Calculate expertise by author
        expertise_map = {}

        # Process file ownership
        for path, authors in ownership['files'].items():
            extension = os.path.splitext(path)[1].lower()
            if not extension:
                extension = 'unknown'
            else:
                extension = extension[1:]  # Remove the leading dot

            for author, line_count in authors.items():
                if author not in expertise_map:
                    expertise_map[author] = {
                        'files': {},
                        'directories': {},
                        'languages': {},
                        'total_lines': 0,
                    }

                expertise = expertise_map[author]
                expertise['files'][path] = line_count
                expertise['total_lines'] += line_count

                # Update language expertise
                if extension not in expertise['languages']:
                    expertise['languages'][extension] = 0
                expertise['languages'][extension] += line_count

        # Process directory ownership
        for dir_path, authors in ownership['directories'].items():
            for author, line_count in authors.items():
                if author in expertise_map:
                    expertise = expertise_map[author]
                    expertise['directories'][dir_path] = line_count

        # Calculate repository contribution percentages
        total_lines = sum(exp['total_lines'] for exp in expertise_map.values())

        for author, expertise in expertise_map.items():
            if total_lines > 0:
                expertise['repository_contribution'] = (expertise['total_lines'] / total_lines) * 100
            else:
                expertise['repository_contribution'] = 0

        return expertise_map