const eslint = require('eslint');
const fs = require('fs');

const args = process.argv.slice(2);
if (args.length < 1) {
    console.error('Uso: node validator.js <archivo.js>');
    process.exit(1);
}

const filePath = args[0];
const code = fs.readFileSync(filePath, 'utf-8');
const cli = new eslint.ESLint({
    useEslintrc: false,
    overrideConfig: {
        env: { browser: true, node: true, es2021: true },
        parserOptions: { ecmaVersion: 12, sourceType: 'module' },
        rules: {
            'no-unused-vars': 'warn',
            'no-undef': 'error',
            'semi': ['error', 'always'],
            'quotes': ['warn', 'single'],
            'complexity': ['warn', 10],
            'no-eval': 'error',
        }
    }
});

cli.lintText(code, { filePath }).then(results => {
    const errors = results[0].messages.map(msg => ({
        message: msg.message,
        line: msg.line,
        column: msg.column,
        ruleId: msg.ruleId
    }));
    console.log(JSON.stringify(errors));
}).catch(err => {
    console.error(JSON.stringify([{ message: err.message, line: 0, column: 0, ruleId: 'error' }]));
});