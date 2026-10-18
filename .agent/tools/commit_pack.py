import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("message")
    parser.add_argument("paths", nargs="+")
    arguments = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    assert all((root / path).resolve() != root / "README.md" for path in arguments.paths)
    command = ["git", "-c", "user.name=Codex", "-c", "user.email=codex@openai.com"]

    def git(*parameters, environment=None, data=None):
        return subprocess.check_output(
            command + list(parameters), cwd=root, env=environment, input=data, text=True
        ).strip()

    git("diff", "--cached", "--quiet")
    base = git("rev-parse", "HEAD")
    branch = git("symbolic-ref", "HEAD")
    assert branch.startswith("refs/heads/perf/")
    git_directory = Path(git("rev-parse", "--absolute-git-dir"))
    with tempfile.TemporaryDirectory(prefix="chargeback-", dir=git_directory) as temporary:
        directory = Path(temporary)
        objects = directory / "objects"
        objects.mkdir()
        environment = os.environ | {
            "GIT_INDEX_FILE": str(directory / "index"),
            "GIT_OBJECT_DIRECTORY": str(objects),
            "GIT_ALTERNATE_OBJECT_DIRECTORIES": str(git_directory / "objects"),
        }
        git("read-tree", base, environment=environment)
        git("add", "--", *arguments.paths, environment=environment)
        tree = git("write-tree", environment=environment)
        commit = git(
            "commit-tree", tree, "-p", base,
            environment=environment, data=arguments.message + "\n",
        )
        git(
            "pack-objects", "--revs", str(git_directory / "objects/pack/pack"),
            environment=environment, data=f"{commit}\n^{base}\n",
        )
        git("cat-file", "-e", f"{commit}^{{commit}}")
        assert git("symbolic-ref", "HEAD") == branch
        git("diff", "--cached", "--quiet")
        git("update-ref", branch, commit, base)
        git("reset", "-q", commit, "--", *arguments.paths)
        print(f"{branch}: {commit}")


if __name__ == "__main__":
    main()
