// Docker Tools — insert starter Dockerfile / docker-compose templates.
const DOCKERFILE = `FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
EXPOSE 3000
CMD ["npm", "start"]
`;
const COMPOSE = `services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
`;
export function activate(ide) {
  ide.commands.register("docker.dockerfile", async () => {
    await ide.editor.insertText(DOCKERFILE);
    ide.window.showInformationMessage("Inserted Dockerfile template");
  });
  ide.commands.register("docker.compose", async () => {
    await ide.editor.insertText(COMPOSE);
    ide.window.showInformationMessage("Inserted docker-compose template");
  });
  ide.window.setStatusBarItem("dockerTools", { text: "Docker", tooltip: "Docker Tools", command: "docker.dockerfile" });
}
