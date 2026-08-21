import { BrowserRouter, Route, Routes } from "react-router-dom";
import { LoginPage } from "@/features/auth/login-page";
import { RequireAuth } from "@/features/auth/require-auth";
import { EnvironmentsPage } from "@/features/environments/environments-page";
import { SettingsPage } from "@/features/settings/settings-page";

function App() {
	return (
		<BrowserRouter>
			<Routes>
				<Route path="/login" element={<LoginPage />} />
				<Route element={<RequireAuth />}>
					<Route index element={<EnvironmentsPage />} />
					<Route path="settings" element={<SettingsPage />} />
				</Route>
			</Routes>
		</BrowserRouter>
	);
}

export default App;
