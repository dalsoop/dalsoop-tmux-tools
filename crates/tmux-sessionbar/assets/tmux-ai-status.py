#!/usr/bin/env python3

import json
import os
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path


CACHE_DIR = Path.home() / ".cache" / "tmux-ai-status"
CACHE_FILE = CACHE_DIR / "state.json"
CACHE_TTL_SECONDS = 5
CLAUDE_DIR = Path.home() / ".claude" / "projects"
CODEX_DIR = Path.home() / ".codex" / "sessions"


def run_tmux(*args):
    proc = subprocess.run(
        ["tmux", *args],
        check=False,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        return ""
    return proc.stdout.strip()


def list_all_panes():
    output = run_tmux(
        "list-panes",
        "-a",
        "-F",
        "#{pane_id}\t#{session_name}:#{window_index}\t#{pane_index}\t#{pane_tty}\t#{pane_current_path}\t#{pane_current_command}",
    )
    panes = []
    for line in output.splitlines():
        parts = line.split("\t")
        if len(parts) != 6:
            continue
        pane_id, window_target, pane_index, pane_tty, pane_path, pane_command = parts
        panes.append(
            {
                "pane_id": pane_id,
                "window_target": window_target,
                "pane_index": pane_index,
                "pane_tty": pane_tty,
                "pane_path": pane_path,
                "pane_command": pane_command,
            }
        )
    return panes


def tty_name(pane_tty):
    return pane_tty.replace("/dev/", "", 1)


def provider_from_text(text):
    lowered = text.lower()
    if "claude" in lowered:
        return "claude"
    if "codex" in lowered:
        return "codex"
    return None


def discover_agent_process(pane):
    provider = provider_from_text(pane["pane_command"])
    tty = tty_name(pane["pane_tty"])
    proc = subprocess.run(
        ["ps", "-t", tty, "-o", "pid=,etimes=,comm=,args="],
        check=False,
        capture_output=True,
        text=True,
    )
    candidates = []
    for line in proc.stdout.splitlines():
        parts = line.strip().split(None, 3)
        if len(parts) < 4:
            continue
        pid_text, etimes_text, comm, args = parts
        line_provider = provider_from_text(comm) or provider_from_text(args)
        if line_provider is None:
            continue
        try:
            pid = int(pid_text)
            etimes = int(etimes_text)
        except ValueError:
            continue
        candidates.append(
            {
                "provider": line_provider,
                "pid": pid,
                "etimes": etimes,
            }
        )

    if provider is not None:
        provider_candidates = [item for item in candidates if item["provider"] == provider]
        if provider_candidates:
            candidates = provider_candidates

    if not candidates:
        return None

    candidates.sort(key=lambda item: (item["etimes"], item["pid"]))
    return candidates[0]


def claude_project_dir(cwd):
    return CLAUDE_DIR / cwd.replace("/", "-")


def recent_claude_sessions(cwd, limit):
    project_dir = claude_project_dir(cwd)
    if not project_dir.exists():
        return []
    files = [path for path in project_dir.glob("*.jsonl") if path.is_file()]
    files.sort(key=lambda path: path.stat().st_mtime, reverse=True)
    return files[:limit]


def recent_codex_files(limit=250):
    if not CODEX_DIR.exists():
        return []
    files = []
    for year_dir in CODEX_DIR.iterdir():
        if not year_dir.is_dir():
            continue
        for month_dir in year_dir.iterdir():
            if not month_dir.is_dir():
                continue
            for day_dir in month_dir.iterdir():
                if not day_dir.is_dir():
                    continue
                for path in day_dir.glob("*.jsonl"):
                    if path.is_file():
                        files.append(path)
    files.sort(key=lambda path: path.stat().st_mtime, reverse=True)
    return files[:limit]


def codex_session_cwd(path):
    try:
        with path.open("r", encoding="utf-8") as handle:
            first_line = handle.readline()
    except OSError:
        return None
    if not first_line:
        return None
    try:
        data = json.loads(first_line)
    except json.JSONDecodeError:
        return None
    if data.get("type") != "session_meta":
        return None
    payload = data.get("payload") or {}
    cwd = payload.get("cwd")
    if isinstance(cwd, str):
        return cwd
    return None


def recent_codex_sessions(cwd, limit):
    matches = []
    for path in recent_codex_files():
        session_cwd = codex_session_cwd(path)
        if session_cwd == cwd:
            matches.append(path)
            if len(matches) >= limit:
                break
    return matches


def is_mcp_tool(name):
    lowered = name.lower()
    return name.startswith("mcp__") or lowered.startswith("mcp_") or "mcp" in lowered


def parse_claude_session(path):
    total_tokens = 0
    user_count = 0
    assistant_count = 0
    tool_count = 0
    mcp_count = 0
    counted_assistant_ids = set()
    last_timestamp = int(path.stat().st_mtime)

    try:
        handle = path.open("r", encoding="utf-8")
    except OSError:
        return {
            "provider": "claude",
            "tokens": 0,
            "messages": 0,
            "tools": 0,
            "mcp": 0,
            "updated": last_timestamp,
        }

    with handle:
        for line in handle:
            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue

            entry_type = data.get("type")
            timestamp = data.get("timestamp")
            if isinstance(timestamp, str):
                last_timestamp = max(last_timestamp, parse_timestamp(timestamp))

            if entry_type == "user":
                user_count += 1
                continue

            if entry_type != "assistant":
                continue

            message = data.get("message") or {}
            message_id = message.get("id") or data.get("uuid", "")

            content = message.get("content") or []
            has_text = any(item.get("type") == "text" for item in content if isinstance(item, dict))
            has_stop = message.get("stop_reason") is not None
            for item in content:
                if not isinstance(item, dict) or item.get("type") != "tool_use":
                    continue
                tool_count += 1
                tool_name = item.get("name")
                if isinstance(tool_name, str) and is_mcp_tool(tool_name):
                    mcp_count += 1
            if not has_text and not has_stop:
                continue

            if message_id and message_id in counted_assistant_ids:
                continue
            if message_id:
                counted_assistant_ids.add(message_id)

            assistant_count += 1
            usage = message.get("usage") or {}
            total_tokens = (
                int(usage.get("input_tokens", 0) or 0)
                + int(usage.get("cache_creation_input_tokens", 0) or 0)
                + int(usage.get("cache_read_input_tokens", 0) or 0)
                + int(usage.get("output_tokens", 0) or 0)
            )

    return {
        "provider": "claude",
        "tokens": total_tokens,
        "messages": user_count + assistant_count,
        "tools": tool_count,
        "mcp": mcp_count,
        "updated": last_timestamp,
    }


def parse_codex_session(path):
    total_tokens = 0
    message_count = 0
    tool_count = 0
    mcp_count = 0
    last_timestamp = int(path.stat().st_mtime)

    try:
        handle = path.open("r", encoding="utf-8")
    except OSError:
        return {
            "provider": "codex",
            "tokens": 0,
            "messages": 0,
            "tools": 0,
            "mcp": 0,
            "updated": last_timestamp,
        }

    with handle:
        for line in handle:
            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue

            timestamp = data.get("timestamp")
            if isinstance(timestamp, str):
                last_timestamp = max(last_timestamp, parse_timestamp(timestamp))

            if data.get("type") == "response_item":
                payload = data.get("payload") or {}
                if payload.get("type") == "message" and payload.get("role") in {"user", "assistant"}:
                    message_count += 1
                if payload.get("type") in {"function_call", "custom_tool_call"}:
                    tool_name = payload.get("name")
                    if isinstance(tool_name, str):
                        tool_count += 1
                        if is_mcp_tool(tool_name):
                            mcp_count += 1
                continue

            if data.get("type") != "event_msg":
                continue

            payload = data.get("payload") or {}
            if payload.get("type") != "token_count":
                continue

            info = payload.get("info") or {}
            total_usage = info.get("total_token_usage") or {}
            total_tokens = int(total_usage.get("total_tokens", 0) or 0)

    return {
        "provider": "codex",
        "tokens": total_tokens,
        "messages": message_count,
        "tools": tool_count,
        "mcp": mcp_count,
        "updated": last_timestamp,
    }


def parse_timestamp(value):
    value = value.strip()
    if not value:
        return 0
    if value.endswith("Z"):
        value = value[:-1] + "+00:00"
    try:
        return int(datetime.fromisoformat(value).timestamp())
    except ValueError:
        return 0


def format_tokens(value):
    if value >= 1000000:
        return f"{value / 1000000:.1f}".rstrip("0").rstrip(".") + "m"
    if value >= 10000:
        return f"{value / 1000:.0f}k"
    if value >= 1000:
        return f"{value / 1000:.1f}".rstrip("0").rstrip(".") + "k"
    return str(value)


def format_summary(summary):
    if not summary:
        return "-"
    tokens = format_tokens(summary["tokens"])
    messages = summary["messages"]
    tools = summary.get("tools", 0)
    return f"tok={tokens} msg={messages} use={tools}"


def compute_state():
    panes = list_all_panes()
    pane_records = {}
    groups = {}

    for pane in panes:
        agent_proc = discover_agent_process(pane)
        pane_records[pane["pane_id"]] = {
            "window_target": pane["window_target"],
            "pane_index": pane["pane_index"],
            "pane_path": pane["pane_path"],
            "provider": agent_proc["provider"] if agent_proc else None,
            "etimes": agent_proc["etimes"] if agent_proc else None,
            "summary": None,
        }
        provider = pane_records[pane["pane_id"]]["provider"]
        if provider is None:
            continue
        key = (provider, pane["pane_path"])
        groups.setdefault(key, []).append((pane["pane_id"], pane_records[pane["pane_id"]]))

    for (provider, pane_path), group in groups.items():
        group.sort(key=lambda item: (item[1]["etimes"] if item[1]["etimes"] is not None else 10 ** 9, item[0]))
        session_limit = max(len(group) + 3, 6)
        if provider == "claude":
            candidates = recent_claude_sessions(pane_path, session_limit)
            parser = parse_claude_session
        else:
            candidates = recent_codex_sessions(pane_path, session_limit)
            parser = parse_codex_session

        for entry, session_path in zip(group, candidates):
            pane_id, pane_record = entry
            pane_record["summary"] = parser(session_path)
            pane_records[pane_id] = pane_record

    return {
        "generated_at": int(time.time()),
        "panes": {
            pane_id: {
                "window_target": pane_record["window_target"],
                "pane_index": pane_record["pane_index"],
                "pane_path": pane_record["pane_path"],
                "provider": pane_record["provider"],
                "summary": pane_record["summary"],
                "summary_text": format_summary(pane_record["summary"]) if pane_record["provider"] else "",
            }
            for pane_id, pane_record in pane_records.items()
        },
    }


def load_state():
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    if CACHE_FILE.exists():
        try:
            cached = json.loads(CACHE_FILE.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            cached = None
        if cached and int(time.time()) - int(cached.get("generated_at", 0)) < CACHE_TTL_SECONDS:
            return cached

    state = compute_state()
    temp_path = CACHE_FILE.with_name(f"{CACHE_FILE.stem}.{os.getpid()}.tmp")
    temp_path.write_text(json.dumps(state), encoding="utf-8")
    temp_path.replace(CACHE_FILE)
    return state


def pane_output(pane_id):
    state = load_state()
    pane = state.get("panes", {}).get(pane_id)
    if not pane:
        return ""
    return pane.get("summary_text", "")


def window_output(window_target):
    state = load_state()
    entries = []
    for pane in state.get("panes", {}).values():
        if pane.get("window_target") != window_target:
            continue
        pane_index = pane.get("pane_index")
        summary_text = pane.get("summary_text") or "-"
        entries.append((int(pane_index), f"{pane_index}:{summary_text}"))
    if not entries:
        return ""
    entries.sort(key=lambda item: item[0])
    text = "AI " + " ".join(item[1] for item in entries)
    if len(text) > 200:
        text = text[:197] + "..."
    return text


def main():
    if len(sys.argv) < 3:
        print("")
        return

    mode = sys.argv[1]
    target = sys.argv[2]

    if mode == "pane":
        print(pane_output(target))
        return
    if mode == "window":
        print(window_output(target))
        return

    print("")


if __name__ == "__main__":
    main()
