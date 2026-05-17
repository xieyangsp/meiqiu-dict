import redBrown from '../assets/teddy/teddy-red-brown.png';
import cream from '../assets/teddy/teddy-cream.png';
import white from '../assets/teddy/teddy-white.png';
import black from '../assets/teddy/teddy-black.png';
import darkBrown from '../assets/teddy/teddy-dark-brown.png';
import apricot from '../assets/teddy/teddy-apricot.png';
import gray from '../assets/teddy/teddy-gray.png';
import silver from '../assets/teddy/teddy-silver.png';

export type SkinId =
  | 'red-brown'
  | 'cream'
  | 'white'
  | 'black'
  | 'dark-brown'
  | 'apricot'
  | 'gray'
  | 'silver';

export interface Skin {
  id: SkinId;
  label: string;
  swatch: string;
  teddy: string;
}

export const DEFAULT_SKIN: SkinId = 'gray';

export const SKINS: Skin[] = [
  { id: 'red-brown', label: '红棕色', swatch: '#B5651D', teddy: redBrown },
  { id: 'cream', label: '奶油色', swatch: '#F5DEB3', teddy: cream },
  { id: 'white', label: '白色', swatch: '#FFFFFF', teddy: white },
  { id: 'black', label: '黑色', swatch: '#1A1A1A', teddy: black },
  { id: 'dark-brown', label: '深棕色', swatch: '#5C4033', teddy: darkBrown },
  { id: 'apricot', label: '杏色', swatch: '#EAB785', teddy: apricot },
  { id: 'gray', label: '灰色', swatch: '#808080', teddy: gray },
  { id: 'silver', label: '银色', swatch: '#C0C0C0', teddy: silver },
];

const SKIN_INDEX: Record<SkinId, Skin> = SKINS.reduce(
  (acc, s) => {
    acc[s.id] = s;
    return acc;
  },
  {} as Record<SkinId, Skin>,
);

export function isSkinId(value: string): value is SkinId {
  return value in SKIN_INDEX;
}

export function resolveSkin(value: string | null | undefined): Skin {
  if (value && isSkinId(value)) {
    return SKIN_INDEX[value];
  }
  return SKIN_INDEX[DEFAULT_SKIN];
}
