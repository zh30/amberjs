import { Route, Routes } from "react-router-dom";
import RootLayout from "./routes/__root";
import Home from "./routes/index";
import Docs from "./routes/docs";
import Blog from "./routes/blog";
import Play from "./routes/play";

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<RootLayout />}>
        <Route path="/" element={<Home />} />
        <Route path="/docs" element={<Docs />} />
        <Route path="/docs/:section" element={<Docs />} />
        <Route path="/blog" element={<Blog />} />
        <Route path="/blog/:slug" element={<Blog />} />
        <Route path="/play" element={<Play />} />
      </Route>
    </Routes>
  );
}
