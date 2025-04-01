"""
Advanced analysis features for Git repositories.

This module provides high-level analytical functions for analyzing Git repositories,
including commit history, code ownership, file change frequency, branch divergence,
and other metrics.
"""

import datetime
from dataclasses import dataclass
from typing import List, Dict, Optional, Union, Tuple, Any
from enum import Enum

from .repository import Repository

class TimePeriod(Enum):
    """Time period for aggregating commit data."""
    HOUR = "hour"
    DAY = "day"
    WEEK = "week"
    MONTH = "month"
    YEAR = "year"


@dataclass
class AnalyticsOptions:
    """Options for analytics operations."""
    max_commits: Optional[int] = None
    skip_commits: Optional[int] = None
    include_merges: bool = True
    author_filter: Optional[str] = None
    path_filter: Optional[str] = None
    since_date: Optional[int] = None
    until_date: Optional[int] = None


@dataclass
class FileEvolutionEntry:
    """Information about a file's evolution."""
    commit_hash: str
    author: str
    email: str
    timestamp: int
    message: str
    lines_added: int
    lines_removed: int

    @property
    def date(self) -> datetime.datetime:
        """Return the commit date as a datetime object."""
        return datetime.datetime.fromtimestamp(self.timestamp)


@dataclass
class HotspotEntry:
    """Information about a code hotspot."""
    file_path: str
    churn_factor: int
    complexity: int
    hotspot_factor: float


@dataclass
class AuthorExpertise:
    """Information about an author's expertise."""
    files: Dict[str, int]
    directories: Dict[str, int]
    languages: Dict[str, int]
    total_lines: int
    repository_contribution: float


class Analytics:
    """Analytics for Git repositories."""

    @staticmethod
    def analyze_commit_history_timeline(
            repo: Repository,
            period: TimePeriod = TimePeriod.DAY,
            options: Optional[AnalyticsOptions] = None
    ) -> Dict[str, int]:
        """Analyze the commit activity distribution over time.

        Args:
            repo: The repository to analyze.
            period: The time period to aggregate commits by.
            options: Analytics options.

        Returns:
            A dictionary mapping time periods to commit counts.
        """
        if options is None:
            options = AnalyticsOptions()

        # Convert options to a dictionary for the native implementation
        opts_dict = {
            "max_commits": options.max_commits,
            "skip_commits": options.skip_commits,
            "include_merges": options.include_merges,
            "author_filter": options.author_filter,
            "path_filter": options.path_filter,
            "since_date": options.since_date,
            "until_date": options.until_date,
        }

        try:
            # Try to use native implementation if available
            if hasattr(repo._repo, "analyze_commit_history_timeline"):
                return repo._repo.analyze_commit_history_timeline(period.value, opts_dict)
        except Exception as e:
            print(f"Warning: Native implementation failed, falling back to Python: {e}")

        # Fall back to Python implementation
        args = ["log", "--pretty=format:%at"]

        if not options.include_merges:
            args.append("--no-merges")

        if options.author_filter:
            args.extend(["--author", options.author_filter])

        if options.since_date:
            args.extend(["--since", f"@{options.since_date}"])

        if options.until_date:
            args.extend(["--until", f"@{options.until_date}"])

        if options.max_commits:
            args.extend(["--max-count", str(options.max_commits)])

        if options.skip_commits:
            args.extend(["--skip", str(options.skip_commits)])

        if options.path_filter:
            args.extend(["--", options.path_filter])

        output = repo._repo._run_git_command(args)
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
    def _format_time_period(timestamp: int, period: TimePeriod) -> str:
        """Format a timestamp according to the specified time period."""
        dt = datetime.datetime.fromtimestamp(timestamp)

        if period == TimePeriod.HOUR:
            return dt.strftime('%Y-%m-%d %H:00')
        elif period == TimePeriod.DAY:
            return dt.strftime('%Y-%m-%d')
        elif period == TimePeriod.WEEK:
            # Calculate ISO week number
            week_number = dt.isocalendar()[1]
            return f"{dt.year}-W{week_number:02d}"
        elif period == TimePeriod.MONTH:
            return dt.strftime('%Y-%m')
        elif period == TimePeriod.YEAR:
            return dt.strftime('%Y')
        else:
            raise ValueError(f"Unknown period: {period}")

    @staticmethod
    def analyze_file_evolution(
            repo: Repository,
            file_path: str,
            options: Optional[AnalyticsOptions] = None
    ) -> List[FileEvolutionEntry]:
        """Analyze the evolution of a file over time.

        Args:
            repo: The repository to analyze.
            file_path: The path to the file to analyze.
            options: Analytics options.

        Returns:
            A list of file evolution entries.
        """
        if options is None:
            options = AnalyticsOptions()

        # Convert options to a dictionary for the native implementation
        opts_dict = {
            "max_commits": options.max_commits,
            "skip_commits": options.skip_commits,
            "include_merges": options.include_merges,
            "author_filter": options.author_filter,
            "since_date": options.since_date,
            "until_date": options.until_date,
        }

        try:
            # Try to use native implementation if available
            if hasattr(repo._repo, "analyze_file_evolution"):
                results = repo._repo.analyze_file_evolution(file_path, opts_dict)
                return [FileEvolutionEntry(**entry) for entry in results]
        except Exception as e:
            print(f"Warning: Native implementation failed, falling back to Python: {e}")

        # Fall back to Python implementation
        args = ["log", "--follow", "--patch", "--pretty=format:%H%n%an%n%ae%n%at%n%s"]

        if options.max_commits:
            args.extend(["--max-count", str(options.max_commits)])

        args.extend(["--", file_path])

        output = repo._repo._run_git_command(args)

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

                    evolution.append(FileEvolutionEntry(
                        commit_hash=current_commit,
                        author=current_author,
                        email=current_email,
                        timestamp=current_timestamp,
                        message=current_message,
                        lines_added=lines_added,
                        lines_removed=lines_removed,
                    ))

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

            evolution.append(FileEvolutionEntry(
                commit_hash=current_commit,
                author=current_author,
                email=current_email,
                timestamp=current_timestamp,
                message=current_message,
                lines_added=lines_added,
                lines_removed=lines_removed,
            ))

        return evolution

    @staticmethod
    def analyze_code_churn(
            repo: Repository,
            options: Optional[AnalyticsOptions] = None
    ) -> Dict[str, Dict[str, Any]]:
        """Analyze code churn in the repository.

        Args:
            repo: The repository to analyze.
            options: Analytics options.

        Returns:
            A dictionary mapping file paths to churn metrics.
        """
        if options is None:
            options = AnalyticsOptions()

        # Convert options to a dictionary for the native implementation
        opts_dict = {
            "max_commits": options.max_commits,
            "skip_commits": options.skip_commits,
            "include_merges": options.include_merges,
            "author_filter": options.author_filter,
            "path_filter": options.path_filter,
            "since_date": options.since_date,
            "until_date": options.until_date,
        }

        try:
            # Try to use native implementation if available
            if hasattr(repo._repo, "analyze_code_churn"):
                return repo._repo.analyze_code_churn(opts_dict)
        except Exception as e:
            print(f"Warning: Native implementation failed, falling back to Python: {e}")

        # Fall back to Python implementation
        args = ["log", "--name-only", "--pretty=format:%H"]

        if not options.include_merges:
            args.append("--no-merges")

        if options.author_filter:
            args.extend(["--author", options.author_filter])

        if options.since_date:
            args.extend(["--since", f"@{options.since_date}"])

        if options.until_date:
            args.extend(["--until", f"@{options.until_date}"])

        if options.max_commits:
            args.extend(["--max-count", str(options.max_commits)])

        if options.path_filter:
            args.extend(["--", options.path_filter])

        output = repo._repo._run_git_command(args)

        churn = {}
        current_commit = None

        for line in output.splitlines():
            if not line.strip():
                continue

            if current_commit is None:
                current_commit = line
            else:
                # This is a file path
                if line not in churn:
                    churn[line] = {
                        "change_count": 0,
                        "commits": set(),
                    }

                churn_metrics = churn[line]
                churn_metrics["change_count"] += 1
                churn_metrics["commits"].add(current_commit)

        # Get more details for frequently changed files
        frequency_data = repo.analyze_change_frequency()

        for entry in frequency_data:
            if entry.path in churn:
                churn[entry.path].update({
                    "last_modified": entry.last_modified,
                    "authors": entry.authors,
                    "primary_owner": entry.primary_owner,
                })

        return churn

    @staticmethod
    def analyze_hotspots(
            repo: Repository,
            options: Optional[AnalyticsOptions] = None
    ) -> List[HotspotEntry]:
        """Identify code hotspots in the repository.

        Args:
            repo: The repository to analyze.
            options: Analytics options.

        Returns:
            A list of hotspot entries.
        """
        if options is None:
            options = AnalyticsOptions()

        # Convert options to a dictionary for the native implementation
        opts_dict = {
            "max_commits": options.max_commits,
            "skip_commits": options.skip_commits,
            "include_merges": options.include_merges,
            "author_filter": options.author_filter,
            "path_filter": options.path_filter,
            "since_date": options.since_date,
            "until_date": options.until_date,
        }

        try:
            # Try to use native implementation if available
            if hasattr(repo._repo, "analyze_hotspots"):
                results = repo._repo.analyze_hotspots(opts_dict)
                return [HotspotEntry(**entry) for entry in results]
        except Exception as e:
            print(f"Warning: Native implementation failed, falling back to Python: {e}")

        # Fall back to Python implementation
        # Get change frequency data
        frequency_data = repo.analyze_change_frequency()

        # Get tracked files
        tracked_files = repo.list_tracked()

        # Filter files
        source_extensions = ['.rs', '.py', '.java', '.js', '.ts', '.c', '.cpp', '.h', '.go', '.rb', '.php', '.cs']

        def is_source_file(path):
            ext = path.lower().split('.')[-1] if '.' in path else ''
            return f".{ext}" in source_extensions and '/node_modules/' not in path and '/vendor/' not in path

        source_files = [f for f in tracked_files if is_source_file(f)]

        # Calculate complexity and hotspot factor
        hotspots = []
        for file_path in source_files:
            # Find file in frequency data
            frequency_entry = next((f for f in frequency_data if f.path == file_path), None)
            if frequency_entry is None:
                continue

            churn_factor = frequency_entry.change_count

            # Simple complexity heuristic based on file size
            try:
                if hasattr(repo._repo, "_run_git_command"):
                    content = repo._repo._run_git_command(["show", f"HEAD:{file_path}"])
                    line_count = len(content.splitlines())
                    complexity = line_count
                else:
                    # Estimate based on change count
                    complexity = churn_factor * 10
            except:
                complexity = 0

            # Calculate hotspot factor
            hotspot_factor = churn_factor * (complexity ** 0.5) / 1000.0

            if hotspot_factor > 0:
                hotspots.append(HotspotEntry(
                    file_path=file_path,
                    churn_factor=churn_factor,
                    complexity=complexity,
                    hotspot_factor=hotspot_factor,
                ))

        # Sort by hotspot factor (descending)
        hotspots.sort(key=lambda x: x.hotspot_factor, reverse=True)

        return hotspots

    @staticmethod
    def analyze_knowledge_map(
            repo: Repository,
            options: Optional[AnalyticsOptions] = None
    ) -> Dict[str, AuthorExpertise]:
        """Analyze the knowledge map of the repository.

        Args:
            repo: The repository to analyze.
            options: Analytics options.

        Returns:
            A dictionary mapping authors to their expertise areas.
        """
        if options is None:
            options = AnalyticsOptions()

        # Convert options to a dictionary for the native implementation
        opts_dict = {
            "max_commits": options.max_commits,
            "skip_commits": options.skip_commits,
            "include_merges": options.include_merges,
            "author_filter": options.author_filter,
            "path_filter": options.path_filter,
            "since_date": options.since_date,
            "until_date": options.until_date,
        }

        try:
            # Try to use native implementation if available
            if hasattr(repo._repo, "analyze_knowledge_map"):
                results = repo._repo.analyze_knowledge_map(opts_dict)
                return {
                    author: AuthorExpertise(**expertise)
                    for author, expertise in results.items()
                }
        except Exception as e:
            print(f"Warning: Native implementation failed, falling back to Python: {e}")

        # Fall back to Python implementation
        # Get code ownership data
        ownership = repo.analyze_code_ownership()

        # Calculate expertise by author
        expertise_map = {}

        # Process file ownership
        for path, authors in ownership['files'].items():
            extension = path.lower().split('.')[-1] if '.' in path else 'unknown'

            for author, line_count in authors.items():
                if author not in expertise_map:
                    expertise_map[author] = {
                        'files': {},
                        'directories': {},
                        'languages': {},
                        'total_lines': 0,
                        'repository_contribution': 0.0,
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
        total_lines = sum(data['total_lines'] for data in expertise_map.values())

        for author, expertise in expertise_map.items():
            if total_lines > 0:
                expertise['repository_contribution'] = (expertise['total_lines'] / total_lines) * 100
            else:
                expertise['repository_contribution'] = 0

        # Convert to AuthorExpertise objects
        return {
            author: AuthorExpertise(
                files=data['files'],
                directories=data['directories'],
                languages=data['languages'],
                total_lines=data['total_lines'],
                repository_contribution=data['repository_contribution'],
            )
            for author, data in expertise_map.items()
        }