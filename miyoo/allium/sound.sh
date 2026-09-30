#!/bin/sh
# Sound plumbing for Kartu on Allium. Sourced by launch.sh and play.sh.
#
# kartu is static and writes /dev/dsp itself, so the audioserver Allium runs for its RetroArch
# cores has to get out of the way first. Two things bite on this hardware:
#  - opening /dev/dsp re-creates the AO device UNMUTED AT 0 dB, and a restarted audioserver comes
#    back at -3 dB unmuted: neither knows the system volume. So we hand kartu the level (it applies
#    it before the first sample) and put the level back after the audioserver returns;
#  - every teardown/creation of the device toggles the speaker amp, which pops. When the system is
#    muted we therefore don't touch audio at all: the audioserver stays, kartu runs with --mute.
#
# Allium keeps the level in /tmp/volume as dB (-60 = its volume 0, which it also mutes).
AO=/proc/mi_modules/mi_ao/mi_ao0
STOP_AS=/mnt/SDCARD/.tmp_update/script/stop_audioserver.sh
START_AS=/mnt/SDCARD/.tmp_update/script/start_audioserver.sh
# start_audioserver.sh finds audioserver_* through PATH and its libs (as_preload.so, libmi_*)
# through LD_LIBRARY_PATH; Allium's own launch env has both, an ssh shell doesn't.
PATH=/mnt/SDCARD/.tmp_update/bin:$PATH
case ":$LD_LIBRARY_PATH:" in
*:/customer/lib:*) ;;
*) LD_LIBRARY_PATH=/lib:/config/lib:/customer/lib:/mnt/SDCARD/miyoo:/mnt/SDCARD/.tmp_update/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH} ;;
esac
export PATH LD_LIBRARY_PATH

system_volume() { # prints the level in dB; "" when unknown
  cat /tmp/volume 2>/dev/null | tr -cd '0-9-'
}

system_muted() { # true when the system is at volume 0 (Allium mutes the device then)
  vol=$(system_volume)
  [ -n "$vol" ] && [ "$vol" -le -60 ] && return 0
  [ -e "$AO" ] && grep -A1 '^AoDev.*bMute' "$AO" | tail -1 | awk '{exit !($6 == 1)}' && return 0
  return 1
}

# sound_begin: sets KARTU_SOUND to the args kartu needs ("--mute" or "") and prepares the device
sound_begin() {
  if system_muted; then
    KARTU_SOUND=--mute
    return 0
  fi
  KARTU_SOUND=
  vol=$(system_volume)
  [ -n "$vol" ] || vol=-9 # same fallback as Allium's set_sound_level.sh
  export KARTU_AO_VOLUME_DB=$vol
  "$STOP_AS" >/dev/null 2>&1
}

# sound_end: gives the device back to the audioserver at the system level
sound_end() {
  [ -n "$KARTU_SOUND" ] && return 0 # never touched it
  "$START_AS" >/dev/null 2>&1
  # the audioserver creates the device a little after it starts; wait for it (like Allium's ffplay)
  i=0
  while [ ! -e "$AO" ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i + 1)); done
  [ -e "$AO" ] || return 1
  vol=$(system_volume)
  [ -n "$vol" ] || vol=-9
  if [ "$vol" -le -60 ]; then
    echo "set_ao_volume 0 -60dB" >"$AO"
    echo "set_ao_volume 1 -60dB" >"$AO"
    echo "set_ao_mute 1" >"$AO"
  else
    echo "set_ao_volume 0 ${vol}dB" >"$AO"
    echo "set_ao_volume 1 ${vol}dB" >"$AO"
    echo "set_ao_mute 0" >"$AO"
  fi
}
