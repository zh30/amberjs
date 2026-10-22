import { defineWranglerConfig } from "wrangler/experimental-config";

export default defineWranglerConfig({
	build: {
		command: "cp ../../install.sh public/install.sh && cp ../../install.ps1 public/install.ps1 && npx vite build",
		watchDir: "src",
	},
	assetsDirectory: "./dist",
});
