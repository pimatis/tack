const turkishMap: Record<string, string> = {
	İ: 'I',
	Ş: 'S',
	Ç: 'C',
	Ğ: 'G',
	Ü: 'U',
	Ö: 'O',
	ı: 'I',
	i: 'I',
	ş: 'S',
	ç: 'C',
	ğ: 'G',
	ü: 'U',
	ö: 'O'
};

const vowels = new Set(['A', 'E', 'I', 'O', 'U']);

export const PREFIX_MAX_LENGTH = 4;
export const PREFIX_MIN_LENGTH = 2;
export const PREFIX_PATTERN = /^[A-Z0-9]{2,4}$/;

export function generatePrefix(name: string): string {
	const chars = name
		.split('')
		.map((ch) => turkishMap[ch] ?? ch)
		.join('')
		.toUpperCase()
		.split('')
		.filter((ch) => /^[A-Z0-9]$/.test(ch));
	const consonants = chars.filter((ch) => !vowels.has(ch));
	return (consonants.length >= PREFIX_MIN_LENGTH ? consonants : chars)
		.slice(0, PREFIX_MAX_LENGTH)
		.join('');
}

export function normalizePrefix(prefix: string): string {
	return prefix.trim().toUpperCase();
}

export function isValidPrefix(prefix: string): boolean {
	return PREFIX_PATTERN.test(prefix);
}
