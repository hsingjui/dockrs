import { BrowserRouter, Route, Routes } from "react-router-dom";
import { AppLayout } from "@/components/layout/app-layout";
import { EnvironmentsPage } from "@/features/environments/environments-page";

function App() {
	return (
		<BrowserRouter>
			<Routes>
				<Route element={<AppLayout />}>
					<Route index element={<EnvironmentsPage />} />
				</Route>
			</Routes>
		</BrowserRouter>
	);
}

export default App;
