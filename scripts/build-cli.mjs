import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

const target =
	process.env.TAURI_ENV_TARGET_TRIPLE ??
	execFileSync('rustc', ['--print', 'host-tuple'], { encoding: 'utf8' }).trim();

execFileSync(
	'cargo',
	['build', '--release', '--bin', 'tack', '--manifest-path', 'src-tauri/Cargo.toml', '--target', target],
	{ stdio: 'inherit' }
);

const extension = process.platform === 'win32' ? '.exe' : '';
const source = join('src-tauri', 'target', target, 'release', `tack${extension}`);
const destination = join('src-tauri', 'bin', `tack-cli-${target}${extension}`);

mkdirSync(join('src-tauri', 'bin'), { recursive: true });
copyFileSync(source, destination);
