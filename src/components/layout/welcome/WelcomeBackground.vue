<template>
  <div class="welcome-bg">
    <div class="welcome-bg__gradient" />
    <div class="welcome-bg__shapes">
      <div
        v-for="i in 6"
        :key="i"
        class="welcome-bg__shape"
        :style="{
          '--delay': `${i * 0.5}s`,
          '--size': `${60 + Math.random() * 80}px`,
          '--x': `${10 + Math.random() * 80}%`,
          '--y': `${10 + Math.random() * 80}%`,
          '--duration': `${15 + Math.random() * 10}s`
        }"
      />
    </div>
    <div class="welcome-bg__grid" />
  </div>
</template>

<style scoped>
.welcome-bg {
  position: absolute;
  inset: 0;
  pointer-events: none;
  overflow: hidden;
}

.welcome-bg__gradient {
  position: absolute;
  inset: 0;
  background: radial-gradient(
    ellipse 80% 50% at 50% -20%,
    var(--color-primary-light) 0%,
    transparent 50%
  );
  opacity: 0.6;
}

[data-theme='dark'] .welcome-bg__gradient {
  opacity: 0.3;
}

.welcome-bg__shapes {
  position: absolute;
  inset: 0;
}

.welcome-bg__shape {
  position: absolute;
  width: var(--size);
  height: var(--size);
  left: var(--x);
  top: var(--y);
  border-radius: 50%;
  background: linear-gradient(
    135deg,
    var(--color-primary) 0%,
    var(--color-accent) 100%
  );
  opacity: 0.08;
  filter: blur(40px);
  animation: welcome-float-shape var(--duration) ease-in-out infinite;
  animation-delay: var(--delay);
}

[data-theme='dark'] .welcome-bg__shape {
  opacity: 0.05;
}

.welcome-bg__grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(var(--color-border) 1px, transparent 1px),
    linear-gradient(90deg, var(--color-border) 1px, transparent 1px);
  background-size: 60px 60px;
  opacity: 0.3;
  mask-image: radial-gradient(ellipse 70% 70% at 50% 50%, black 20%, transparent 70%);
}

[data-theme='dark'] .welcome-bg__grid {
  opacity: 0.15;
}

@keyframes welcome-float-shape {
  0%, 100% {
    transform: translate(0, 0) scale(1);
  }
  25% {
    transform: translate(20px, -30px) scale(1.1);
  }
  50% {
    transform: translate(-10px, 20px) scale(0.9);
  }
  75% {
    transform: translate(30px, 10px) scale(1.05);
  }
}
</style>
