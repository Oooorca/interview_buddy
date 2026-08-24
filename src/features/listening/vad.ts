export type VadAction = "flush" | "discard" | null;

export type VadState = {
  initialized: boolean;
  speaking: boolean;
  speechMs: number;
  silenceMs: number;
  utteranceMs: number;
  idleMs: number;
  noiseFloor: number;
  lastAt: number;
};

const VAD_ENERGY_FLOOR = 0.0025;
const VAD_SILENCE_MS = 800;
const VAD_MIN_SPEECH_MS = 350;
const VAD_MAX_UTTERANCE_MS = 25_000;
const VAD_IDLE_RESET_MS = 8_000;

export const VAD_POLL_MS = 100;

export function freshVadState(): VadState {
  return {
    initialized: false,
    speaking: false,
    speechMs: 0,
    silenceMs: 0,
    utteranceMs: 0,
    idleMs: 0,
    noiseFloor: VAD_ENERGY_FLOOR,
    lastAt: performance.now(),
  };
}

export function advanceVad(state: VadState, level: number, now: number): VadAction {
  const elapsed = Math.max(40, Math.min(500, now - state.lastAt));
  state.lastAt = now;
  if (!state.initialized) {
    state.noiseFloor = Math.min(level, VAD_ENERGY_FLOOR);
    state.initialized = true;
  }
  const enterThreshold = Math.max(VAD_ENERGY_FLOOR, state.noiseFloor * 2.6);
  const exitThreshold = Math.max(VAD_ENERGY_FLOOR * 0.7, state.noiseFloor * 1.6);
  const voiced = level >= (state.speaking ? exitThreshold : enterThreshold);

  if (!state.speaking) {
    if (voiced) {
      state.speaking = true;
      state.speechMs = elapsed;
      state.silenceMs = 0;
      state.utteranceMs = elapsed;
      state.idleMs = 0;
    } else {
      state.noiseFloor = state.noiseFloor * 0.95 + level * 0.05;
      state.idleMs += elapsed;
      if (state.idleMs >= VAD_IDLE_RESET_MS) {
        Object.assign(state, freshVadState());
        return "discard";
      }
    }
    return null;
  }

  state.utteranceMs += elapsed;
  if (voiced) {
    state.speechMs += elapsed;
    state.silenceMs = 0;
  } else {
    state.silenceMs += elapsed;
  }
  const naturalPause = state.silenceMs >= VAD_SILENCE_MS;
  const hasSpeech = state.speechMs >= VAD_MIN_SPEECH_MS;
  if ((naturalPause && hasSpeech) || state.utteranceMs >= VAD_MAX_UTTERANCE_MS) {
    Object.assign(state, freshVadState());
    return "flush";
  }
  if (naturalPause && !hasSpeech) {
    Object.assign(state, freshVadState());
    return "discard";
  }
  return null;
}
