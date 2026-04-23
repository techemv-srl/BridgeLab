import type { ManualSection } from '../helpContent';
import { itPart1 } from './it-part1';
import { itPart2 } from './it-part2';
import { itPart3 } from './it-part3';

export const itSections: ManualSection[] = [
	...itPart1,
	...itPart2,
	...itPart3,
];
