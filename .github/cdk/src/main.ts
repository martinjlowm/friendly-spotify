import { App, Job, Stack, Workflow } from "cdkactions";
import { Construct } from "constructs";

export class Main extends Stack {
  constructor(scope: Construct, name: string) {
    super(scope, name);

    const workflowName = this.constructor.name.toLowerCase();
    const workflow = new Workflow(this, workflowName, {
      name: workflowName,
      on: {
        schedule: [
          {
            cron: "5 20,21,22,23 * * *",
          },
        ],
      },
    });

    new Job(workflow, "ci", {
      runsOn: "ubuntu-latest",
      timeoutMinutes: 60,
      steps: [
        {
          uses: "actions/checkout@v3",
        },
        {
          name: "Install rust toolchain",
          uses: "actions-rs/toolchain@v1",
        },
        {
          name: "Execute!",
          id: "main",
          run: "cargo run",
          env: {
            RUST_LOG: "trace",
            TOKEN: "${{ secrets.TOKEN }}",
            RSPOTIFY_CLIENT_ID: "${{ secrets.RSPOTIFY_CLIENT_ID }}",
            RSPOTIFY_CLIENT_SECRET: "${{ secrets.RSPOTIFY_CLIENT_SECRET }}",
          },
        },
        {
          name: "Update token",
          uses: "hmanzur/actions-set-secret@v1.0.0",
          with: {
            name: "TOKEN",
            value: "${{ steps.main.outputs.TOKEN }}",
            token: "${{ secrets.PAT }}",
          },
        },
      ],
    });
  }
}

const app = new App();
new Main(app, "cdk");
app.synth();
