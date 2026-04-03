<script setup lang="ts">
import PomodoroPanel from "./components/pomodoro/PomodoroPanel.vue";
</script>

<template>
  <PomodoroPanel />
</template>

<style>
:root {
  --ink-strong: #2f251f;
  --ink-soft: rgba(47, 37, 31, 0.68);
  --accent-muted: #7b4930;

  font-family: "Avenir Next", "Segoe UI", sans-serif;
  line-height: 1.5;
  font-weight: 400;
  color: var(--ink-strong);
  background: #efe3d2;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

html,
body,
#app {
  min-height: 100%;
}

body {
  margin: 0;
}

button,
input {
  font: inherit;
}
</style>
