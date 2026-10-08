<!-- Browser-mock only: stand-ins for the native macOS traffic lights so screenshots show the real
     composition. In Tauri the native ones are drawn by the system in exactly this spot. -->
<div class="lights" aria-hidden="true">
  <span class="red"></span><span class="yellow"></span><span class="green"></span>
</div>

<style>
  .lights {
    position: fixed;
    top: 15px;
    left: 14px;
    z-index: 700;
    display: flex;
    gap: 8px;
    pointer-events: none;
  }
  span {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 0.5px rgb(0 0 0 / 0.18);
  }
  .red {
    background: #ff5f57;
  }
  .yellow {
    background: #febc2e;
  }
  .green {
    background: #28c840;
  }
</style>
