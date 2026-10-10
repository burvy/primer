// Ada's voice and the book's sounds.
//
// The one piece of JavaScript in the project. Browsers' audio features are
// plain function calls on browser objects, which are shorter to call from
// here than through Rust bindings. The Rust page (src/audio.rs) calls these
// five functions and nothing else:
//
//   configure(narration, sounds, music)  what the child switched on
//   say(name, speech)                    Ada reads one line
//   stop()                               Ada stops reading
//   sound(name)                          one short sound
//
// Nothing plays until the child's first tap (browsers forbid it), so a line
// asked for before that waits and is read at the first tap.
//
// To read a line, the files are tried in this order, and the first one that
// exists is played:
//   1. /assets/voice/<name>.ogg, .mp3, .wav     a real recording
//   2. /assets/voice/placeholder/<name>.wav     a computer-voiced stand-in
//   3. the browser's own speech, in a female English voice if there is one
//
// While a line plays, <body> has the class "speaking" (Ada can blink and
// talk to it later).

(function () {
  "use strict";

  const settings = { narration: true, sounds: true, music: false };
  let unlocked = false; // true after the first tap
  let context = null; // WebAudio, for the stand-in sounds
  let current = null; // the line being read now, or null
  let waiting = null; // a line asked for before the first tap
  let music = null; // the looping music element
  let ambience = null; // the looping room-sound element
  const absent = new Set(); // addresses the server has no file for

  const address = (name) => encodeURIComponent(name);

  function setSpeaking(on) {
    document.body.classList.toggle("speaking", on);
  }

  // ---- reading a line aloud -------------------------------------------

  function say(name, speech) {
    stop();
    if (!settings.narration) return;
    if (!unlocked) {
      waiting = { name, speech };
      return;
    }
    const line = { audio: null };
    current = line;
    const files = ["ogg", "mp3", "wav"]
      .map((kind) => `/assets/voice/${address(name)}.${kind}`)
      .concat(`/assets/voice/placeholder/${address(name)}.wav`);
    playFirst(files, line, speech);
  }

  // Plays the first file not known to be missing; speaks when none is left.
  function playFirst(files, line, speech) {
    const file = files.find((f) => !absent.has(f));
    if (!file) {
      speakAloud(speech, line);
      return;
    }
    const audio = new Audio(file);
    line.audio = audio;
    audio.onplaying = () => current === line && setSpeaking(true);
    audio.onended = () => finish(line);
    audio.onerror = () => {
      absent.add(file);
      if (current === line) playFirst(files, line, speech);
    };
    audio.play().catch(() => {}); // a missing file is handled by onerror
  }

  function speakAloud(speech, line) {
    if (!("speechSynthesis" in window)) return finish(line);
    const utterance = new SpeechSynthesisUtterance(speech);
    const voice = pickVoice();
    if (voice) {
      utterance.voice = voice;
      utterance.lang = voice.lang;
    } else {
      utterance.lang = "en-GB";
    }
    utterance.rate = 0.95;
    utterance.onstart = () => current === line && setSpeaking(true);
    utterance.onend = utterance.onerror = () => finish(line);
    window.speechSynthesis.cancel();
    window.speechSynthesis.speak(utterance);
  }

  // A female English voice if the browser has one, else any English voice.
  const FEMALE = /zira|hazel|susan|libby|sonia|jenny|aria|samantha|karen|moira|tessa|serena|female/i;
  function pickVoice() {
    const english = window.speechSynthesis
      .getVoices()
      .filter((v) => v.lang.toLowerCase().startsWith("en"));
    return english.find((v) => FEMALE.test(v.name)) || english[0] || null;
  }

  function finish(line) {
    if (current === line) {
      current = null;
      setSpeaking(false);
    }
  }

  function stop() {
    waiting = null;
    const line = current;
    current = null;
    if (line && line.audio) line.audio.pause();
    if ("speechSynthesis" in window) window.speechSynthesis.cancel();
    setSpeaking(false);
  }

  // ---- sounds and music -----------------------------------------------

  // A short sound: the file in /assets/sounds if there is one, else a
  // little tune made on the spot.
  function sound(name) {
    if (!settings.sounds || !unlocked) return;
    const file = `/assets/sounds/${address(name)}.ogg`;
    if (absent.has(file)) return standIn(name);
    const audio = new Audio(file);
    audio.onerror = () => {
      absent.add(file);
      standIn(name);
    };
    audio.play().catch(() => {});
  }

  // One note. `slideTo` bends the pitch, for the hop.
  function tone(pitch, start, length, shape, loudness, slideTo) {
    if (!context) return;
    const now = context.currentTime + start;
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    oscillator.type = shape;
    oscillator.frequency.setValueAtTime(pitch, now);
    if (slideTo) oscillator.frequency.linearRampToValueAtTime(slideTo, now + length);
    gain.gain.setValueAtTime(loudness, now);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + length);
    oscillator.connect(gain).connect(context.destination);
    oscillator.start(now);
    oscillator.stop(now + length);
  }

  // A soft rustle, like a page.
  function rustle(length) {
    if (!context) return;
    const size = Math.floor(context.sampleRate * length);
    const buffer = context.createBuffer(1, size, context.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < size; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / size);
    const source = context.createBufferSource();
    const filter = context.createBiquadFilter();
    const gain = context.createGain();
    filter.type = "bandpass";
    filter.frequency.value = 2500;
    gain.gain.value = 0.25;
    source.buffer = buffer;
    source.connect(filter).connect(gain).connect(context.destination);
    source.start();
  }

  function standIn(name) {
    if (name === "page-turn") rustle(0.2);
    else if (name === "coin") {
      tone(1320, 0, 0.08, "square", 0.05);
      tone(1760, 0.07, 0.3, "square", 0.05);
    } else if (name === "hop") tone(300, 0, 0.14, "sine", 0.18, 520);
    else if (name === "chime") {
      [523.25, 659.25, 783.99].forEach((pitch, i) => tone(pitch, i * 0.12, 0.6, "sine", 0.14));
    }
    // Any other name stays silent.
  }

  // The loops: no stand-in, so without a file they are simply silent.
  function loop(element, name, loudness) {
    if (element) return element;
    const audio = new Audio(`/assets/sounds/${address(name)}.ogg`);
    audio.loop = true;
    audio.volume = loudness;
    audio.onerror = () => {}; // no file yet: stay quiet
    audio.play().catch(() => {});
    return audio;
  }

  function applyLoops() {
    if (unlocked && settings.music) music = loop(music, "music", 0.35);
    else if (music) music.pause(), (music = null);
    if (unlocked && settings.sounds) ambience = loop(ambience, "ambience", 0.2);
    else if (ambience) ambience.pause(), (ambience = null);
  }

  // ---- the first tap, and the settings --------------------------------

  function unlock() {
    if (unlocked) return;
    unlocked = true;
    try {
      context = new (window.AudioContext || window.webkitAudioContext)();
      context.resume();
    } catch (error) {
      context = null; // no stand-in sounds, files still work
    }
    applyLoops();
    if (waiting) {
      const line = waiting;
      waiting = null;
      say(line.name, line.speech);
    }
  }

  function configure(narration, sounds, musicOn) {
    settings.narration = narration;
    settings.sounds = sounds;
    settings.music = musicOn;
    if (!narration) stop();
    applyLoops();
  }

  document.addEventListener("pointerdown", unlock, { once: true });
  document.addEventListener("keydown", unlock, { once: true });
  // Some browsers load their voice list late; asking once starts the load.
  if ("speechSynthesis" in window) window.speechSynthesis.getVoices();

  window.primerAudio = { configure, say, stop, sound };
})();
