#!/usr/bin/env bash
# Runs fish-tank equilibrium parameter sets as parallel headless simulations
# and ranks the survivors by the smallest population any level ever reached.
#
# Usage: scripts/sweep.sh                 # coarse grid, 180 s per run
#        DURATION=360 SEEDS="4 5 6" scripts/sweep.sh   # refinement runs
#        OUT=sweep-refine scripts/sweep.sh             # keep several sweeps
#
# Each run prints a RESULT line (see log_population in src/sketches/fish_tank.rs);
# runs/ keeps raw logs, and survivors + failures are written to summary.txt.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

FPS=${FPS:-30}
DURATION=${DURATION:-180}
JOBS=${JOBS:-$(nproc)}
SEEDS=${SEEDS:-"1 2"}
RATES=${RATES:-"0.7 1.3 2.2"}
DRAINS=${DRAINS:-"0.5 1.0"}
CATCHES=${CATCHES:-"0.5 1.0 2.0"}
LIVES=${LIVES:-"1.0"}
FLEES=${FLEES:-"0.5 1.0"}
HUNTS=${HUNTS:-"1.0 2.0"}
REACHES=${REACHES:-"1.0 2.0"}
PURSUITS=${PURSUITS:-"1.0 1.2"}
FOUNDERS=${FOUNDERS:-"1.0"}
HUNGERS=${HUNGERS:-"1.0"}
IMMIGRATIONS=${IMMIGRATIONS:-"0"}
OUT=${OUT:-sweep}
BIN=./target/release/kaleidoodles

cargo build --release

rm -rf "$OUT"
mkdir -p "$OUT/runs"

: > "$OUT/commands.txt"
if [ -n "${RANDOM_RUNS:-}" ]; then
  # Random search over the full knob space.
  awk -v runs="$RANDOM_RUNS" -v bin="$BIN" -v fps="$FPS" -v dur="$DURATION" -v out="$OUT" 'BEGIN {
    srand(7);
    for (i = 0; i < runs; i++) {
      rate = 6 + 14 * rand();
      drain = 0.2 + 0.6 * rand();
      cch = 0.2 + 1.2 * rand();
      life = 1.2 + 0.8 * rand();
      flee = 0.1 + 0.5 * rand();
      hunt = 1 + 2.5 * rand();
      reach = 2 + 10 * rand();
      purs = 1.1 + 0.9 * rand();
      found = 3 + 12 * rand();
      hung = 0.4 + 1.2 * rand();
      immig = int(1 + 6 * rand());
      seed = int(1 + 40 * rand());
      name = sprintf("rnd%03d_r%.1f_d%.2f_c%.2f_l%.2f_f%.2f_h%.2f_e%.1f_p%.2f_fo%.1f_hu%.2f_im%d_s%d", i, rate, drain, cch, life, flee, hunt, reach, purs, found, hung, immig, seed);
      printf "%s --simulate --fps %d --duration %d --pellet-rate %.3f --drain-scale %.3f --catch-scale %.3f --lifespan-scale %.3f --flee-scale %.3f --hunt-scale %.3f --reach-scale %.3f --pursuit-scale %.3f --founders-scale %.3f --hunger-scale %.3f --immigration-floor %d --seed %d > %s/runs/%s.log 2>&1 || true\n", bin, fps, dur, rate, drain, cch, life, flee, hunt, reach, purs, found, hung, immig, seed, out, name;
    }
  }' > "$OUT/commands.txt"
else
for rate in $RATES; do
  for drain in $DRAINS; do
    for catch in $CATCHES; do
      for life in $LIVES; do
        for flee in $FLEES; do
          for hunt in $HUNTS; do
            for reach in $REACHES; do
              for pursuit in $PURSUITS; do
                for founders in $FOUNDERS; do
                  for hunger in $HUNGERS; do
                  for immigration in $IMMIGRATIONS; do
                for seed in $SEEDS; do
                  name="r${rate}_d${drain}_c${catch}_l${life}_f${flee}_h${hunt}_e${reach}_p${pursuit}_fo${founders}_hu${hunger}_im${immigration}_s${seed}"
                  echo "$BIN --simulate --fps $FPS --duration $DURATION --pellet-rate $rate --drain-scale $drain --catch-scale $catch --lifespan-scale $life --flee-scale $flee --hunt-scale $hunt --reach-scale $reach --pursuit-scale $pursuit --founders-scale $founders --hunger-scale $hunger --immigration-floor $immigration --seed $seed > $OUT/runs/$name.log 2>&1 || true" >> "$OUT/commands.txt"
                done
                  done
                  done
              done
            done
          done
        done
      done
    done
  done
done
done
fi

total=$(wc -l < "$OUT/commands.txt")
echo "sweeping $total runs across $JOBS parallel jobs..."
xargs -a "$OUT/commands.txt" -P "$JOBS" -I CMD bash -c CMD

grep -h '^RESULT' "$OUT"/runs/*.log > "$OUT/results.txt" || true

awk '
{
  delete kv
  for (i = 1; i <= NF; i++) {
    e = index($i, "=")
    if (e) kv[substr($i, 1, e - 1)] = substr($i, e + 1)
  }
  score = kv["cyan_min"] + 0
  split("lime orange magenta", rest, " ")
  margin = kv["cyan_min"] - kv["immigration"]
  for (n in rest) {
    if (kv[rest[n] "_min"] + 0 < score) score = kv[rest[n] "_min"] + 0
    m = kv[rest[n] "_min"] - kv["immigration"]
    if (m < margin) margin = m
  }
  # Survivors must persist without the insurance safety net, checked directly
  # with the stocked counter rather than inferred from the margin.
  if (kv["extinct"] == "none" && margin > 0 && kv["stocked"] == "0/0/0/0") {
    print margin "\t" score "\t" $0
  } else {
    failures++
  }
}
END { printf "# %d survivors, %d extinct runs\n", NR - failures, failures > "/dev/stderr" }
' "$OUT/results.txt" | sort -k1,1nr > "$OUT/survivors.txt"

echo "--- top survivors (cols: margin-above-floor, min-count, RESULT) ---"
head -n 15 "$OUT/survivors.txt"
