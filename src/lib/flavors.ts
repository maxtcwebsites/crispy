import type { Flavor } from './types';

export interface FlavorInfo {
  id: Flavor;
  name: string;
  /** Swatch colours: chip surface + seasoning dust. */
  color: string;
  dust: string;
}

/** Accent families, named after chip flavours. The colours live in styles/tokens.css. */
export const FLAVORS: FlavorInfo[] = [
  { id: 'classic', name: 'Classic Salted', color: '#F3B544', dust: '#FFF6DA' },
  { id: 'bbq', name: 'Smoky BBQ', color: '#E2733A', dust: '#7A2E10' },
  { id: 'sourCream', name: 'Sour Cream & Onion', color: '#79D3A6', dust: '#2F7A55' },
  { id: 'saltVinegar', name: 'Salt & Vinegar', color: '#7CB8F2', dust: '#F2F8FF' },
  { id: 'sweetChili', name: 'Sweet Chili', color: '#F2545B', dust: '#8C1B22' },
  { id: 'truffle', name: 'Truffle', color: '#B39DF2', dust: '#3D2C6E' },
];

export function flavorInfo(id: string): FlavorInfo {
  return FLAVORS.find((f) => f.id === id) ?? FLAVORS[0];
}
