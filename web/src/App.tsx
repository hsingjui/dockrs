import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";

function App() {
	return (
		<div className="flex min-h-svh items-center justify-center p-6">
			<Card className="max-w-sm">
				<CardHeader>
					<CardTitle>dockrs</CardTitle>
					<CardDescription>Tailwind CSS v4 + shadcn/ui 已接入</CardDescription>
				</CardHeader>
				<CardContent className="flex gap-2">
					<Button>默认按钮</Button>
					<Button variant="outline">Outline</Button>
				</CardContent>
			</Card>
		</div>
	);
}

export default App;
