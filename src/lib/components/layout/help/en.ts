import type { ManualSection } from '../helpContent';
import { getStarted, editorSection, treeSection } from './en-part1';
import { validationSection, communicationSection, anonymizationSection } from './en-part2';
import { fhirSection, pluginsSection, licensingSection, shortcutsSection } from './en-part3';

export const enSections: ManualSection[] = [
	getStarted,
	editorSection,
	treeSection,
	validationSection,
	communicationSection,
	anonymizationSection,
	fhirSection,
	pluginsSection,
	licensingSection,
	shortcutsSection,
];
