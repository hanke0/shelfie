const COVER_WIDTH = 600;
const COVER_HEIGHT = 800;

const COLORS = {
  bgTop: "#e8c4b0",
  bgBottom: "#ebe3d8",
  ink: "#1f1a14",
  inkMuted: "#6b5f52",
  accent: "#b85c38",
};

let fontsReady: Promise<void> | null = null;

function ensureFonts(): Promise<void> {
  if (!fontsReady && typeof document !== "undefined" && document.fonts) {
    fontsReady = Promise.all([
      document.fonts.load('700 52px "Playfair Display"'),
      document.fonts.load('500 28px "DM Sans"'),
    ]).then(() => undefined);
  }
  return fontsReady ?? Promise.resolve();
}

function wrapText(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string[] {
  const trimmed = text.trim();
  if (!trimmed) return [];

  const hasSpaces = /\s/.test(trimmed);
  const units = hasSpaces ? trimmed.split(/\s+/) : [...trimmed];
  const lines: string[] = [];
  let line = "";

  for (const unit of units) {
    const next = line ? (hasSpaces ? `${line} ${unit}` : `${line}${unit}`) : unit;
    if (ctx.measureText(next).width <= maxWidth) {
      line = next;
    } else {
      if (line) lines.push(line);
      line = unit;
    }
  }
  if (line) lines.push(line);
  return lines.slice(0, 6);
}

function drawTitleCover(
  ctx: CanvasRenderingContext2D,
  title: string,
  author?: string,
): void {
  const w = COVER_WIDTH;
  const h = COVER_HEIGHT;

  const gradient = ctx.createLinearGradient(0, 0, 0, h);
  gradient.addColorStop(0, COLORS.bgTop);
  gradient.addColorStop(1, COLORS.bgBottom);
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, w, h);

  ctx.strokeStyle = "rgba(184, 92, 56, 0.25)";
  ctx.lineWidth = 3;
  ctx.strokeRect(24, 24, w - 48, h - 48);

  const pad = 56;
  const maxWidth = w - pad * 2;
  let y = h * 0.28;

  ctx.fillStyle = COLORS.accent;
  ctx.font = '700 52px "Playfair Display", Georgia, serif';
  ctx.textAlign = "center";
  ctx.textBaseline = "top";

  const titleLines = wrapText(ctx, title || "未命名", maxWidth);
  const titleLineHeight = 62;
  for (const line of titleLines) {
    ctx.fillText(line, w / 2, y);
    y += titleLineHeight;
  }

  const authorText = author?.trim();
  if (authorText) {
    y += 20;
    ctx.fillStyle = COLORS.inkMuted;
    ctx.font = '500 28px "DM Sans", system-ui, sans-serif';
    const authorLines = wrapText(ctx, authorText, maxWidth);
    for (const line of authorLines.slice(0, 2)) {
      ctx.fillText(line, w / 2, y);
      y += 36;
    }
  }

  ctx.fillStyle = COLORS.accent;
  ctx.globalAlpha = 0.35;
  ctx.beginPath();
  ctx.arc(w - 80, h - 80, 48, 0, Math.PI * 2);
  ctx.fill();
  ctx.globalAlpha = 1;
}

async function renderCoverCanvas(title: string, author?: string): Promise<HTMLCanvasElement> {
  await ensureFonts();
  const canvas = document.createElement("canvas");
  canvas.width = COVER_WIDTH;
  canvas.height = COVER_HEIGHT;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("无法创建封面画布");
  drawTitleCover(ctx, title, author);
  return canvas;
}

function canvasToBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error("封面生成失败"))),
      "image/jpeg",
      0.92,
    );
  });
}

/** 生成用于展示的 object URL（调用方负责 revoke） */
export async function generateTitleCoverBlobUrl(
  title: string,
  author?: string,
): Promise<string> {
  const canvas = await renderCoverCanvas(title, author);
  const blob = await canvasToBlob(canvas);
  return URL.createObjectURL(blob);
}
