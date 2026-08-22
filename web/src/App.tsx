import { BrowserRouter, Route, Routes } from "react-router-dom";
import { Toaster } from "@/components/ui/toast";
import { LoginPage } from "@/features/auth/login-page";
import { RequireAuth } from "@/features/auth/require-auth";
import { ContainersPage } from "@/features/containers/containers-page";
import { EnvironmentDashboard } from "@/features/environments/environment-dashboard";
import { EnvironmentLayout } from "@/features/environments/environment-layout";
import { EnvironmentsPage } from "@/features/environments/environments-page";
import { ImagesPage } from "@/features/images/images-page";
import { NetworksPage } from "@/features/networks/networks-page";
import { SettingsPage } from "@/features/settings/settings-page";
import { StacksPage } from "@/features/stacks/stacks-page";
import { VolumesPage } from "@/features/volumes/volumes-page";

function App() {
	return (
		<BrowserRouter>
			<Routes>
				<Route path="/login" element={<LoginPage />} />
				<Route element={<RequireAuth />}>
					<Route index element={<EnvironmentsPage />} />
					<Route
						path="environments/:environmentId"
						element={<EnvironmentLayout />}
					>
						<Route index element={<EnvironmentDashboard />} />
						<Route path="stacks" element={<StacksPage />} />
						<Route path="containers" element={<ContainersPage />} />
						<Route path="images" element={<ImagesPage />} />
						<Route path="volumes" element={<VolumesPage />} />
						<Route path="networks" element={<NetworksPage />} />
					</Route>
					<Route path="settings" element={<SettingsPage />} />
				</Route>
			</Routes>
			<Toaster />
		</BrowserRouter>
	);
}

export default App;
