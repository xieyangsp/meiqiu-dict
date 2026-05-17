import { DEFAULT_SKIN, resolveSkin, type SkinId } from './skins';

export function applySkin(skin: SkinId): void {
  document.documentElement.dataset.skin = skin;
}

export function applySkinValue(value: string | null | undefined): SkinId {
  const skin = resolveSkin(value);
  applySkin(skin.id);
  return skin.id;
}

export function applyDefaultSkin(): void {
  applySkin(DEFAULT_SKIN);
}
