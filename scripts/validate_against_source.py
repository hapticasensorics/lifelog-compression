from __future__ import annotations

import argparse
import json
import re
import subprocess
import tempfile
from pathlib import Path


CANVAS_FILTER = (
    "scale=1920:1080:force_original_aspect_ratio=decrease,"
    "pad=1920:1080:(ow-iw)/2:(oh-ih)/2:black"
)

PSNR_RE = re.compile(r"average:(?P<value>[0-9.]+)")
SSIM_RE = re.compile(r"All:(?P<value>[0-9.]+)")


def run(command: list[str]) -> str:
    result = subprocess.run(command, check=True, capture_output=True, text=True)
    return result.stderr + "\n" + result.stdout


def extract_reference_frame(source: Path, timestamp_seconds: float, output: Path) -> None:
    subprocess.run(
        [
            "ffmpeg",
            "-y",
            "-v",
            "error",
            "-ss",
            f"{timestamp_seconds:.6f}",
            "-i",
            str(source),
            "-frames:v",
            "1",
            "-vf",
            CANVAS_FILTER,
            "-q:v",
            "2",
            str(output),
        ],
        check=True,
    )


def psnr(reference: Path, extracted: Path) -> float:
    text = run(
        [
            "ffmpeg",
            "-v",
            "info",
            "-i",
            str(reference),
            "-i",
            str(extracted),
            "-frames:v",
            "1",
            "-filter_complex",
            "[0:v]format=yuv420p[ref];[1:v]format=yuv420p[cmp];[ref][cmp]psnr",
            "-f",
            "null",
            "-",
        ]
    )
    match = PSNR_RE.search(text)
    if not match:
        raise RuntimeError(f"Could not parse PSNR from output:\n{text}")
    return float(match.group("value"))


def ssim(reference: Path, extracted: Path) -> float:
    text = run(
        [
            "ffmpeg",
            "-v",
            "info",
            "-i",
            str(reference),
            "-i",
            str(extracted),
            "-frames:v",
            "1",
            "-filter_complex",
            "[0:v]format=yuv420p[ref];[1:v]format=yuv420p[cmp];[ref][cmp]ssim",
            "-f",
            "null",
            "-",
        ]
    )
    match = SSIM_RE.search(text)
    if not match:
        raise RuntimeError(f"Could not parse SSIM from output:\n{text}")
    return float(match.group("value"))


def summarize(values: list[float]) -> dict[str, float | None]:
    if not values:
        return {"min": None, "mean": None, "max": None}
    return {
        "min": min(values),
        "mean": sum(values) / len(values),
        "max": max(values),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate extracted frames against source material.")
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--bundle-dir", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=0, help="Optional frame limit; 0 means all frames.")
    args = parser.parse_args()

    manifest_path = args.bundle_dir / "manifest.jsonl"
    if not manifest_path.exists():
        print(json.dumps({"error": "missing_manifest", "path": str(manifest_path)}, indent=2))
        return 2

    rows = [
        json.loads(line)
        for line in manifest_path.read_text().splitlines()
        if line.strip()
    ]
    if args.limit > 0:
        rows = rows[: args.limit]

    psnr_values: list[float] = []
    ssim_values: list[float] = []
    per_frame: list[dict[str, object]] = []

    with tempfile.TemporaryDirectory(prefix="lifelog-validate-") as tmpdir:
        tmpdir_path = Path(tmpdir)
        for row in rows:
            frame_name = Path(row["frame_relpath"]).name
            extracted = args.bundle_dir / row["frame_relpath"]
            reference = tmpdir_path / frame_name
            ts = float(row["actual_ts_ms"]) / 1000.0
            extract_reference_frame(args.source, ts, reference)
            psnr_value = psnr(reference, extracted)
            ssim_value = ssim(reference, extracted)
            psnr_values.append(psnr_value)
            ssim_values.append(ssim_value)
            per_frame.append(
                {
                    "filename": frame_name,
                    "requested_seconds": float(row["requested_ts_ms"]) / 1000.0,
                    "actual_seconds": ts,
                    "psnr": psnr_value,
                    "ssim": ssim_value,
                }
            )

    payload = {
        "source": str(args.source),
        "bundle_dir": str(args.bundle_dir),
        "frame_count": len(per_frame),
        "psnr": summarize(psnr_values),
        "ssim": summarize(ssim_values),
        "frames": per_frame,
    }
    print(json.dumps(payload, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
