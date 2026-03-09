#!/usr/bin/env node
/**
 * MoltStack Quality Validator
 * Checks if a draft meets The Protheus Codex standards
 * Usage: node quality-check.js '{"title":"...","content":"..."}'
 */

function qualityCheck(postData) {
  const results = {
    passed: true,
    checks: [],
    warnings: []
  };

  // Check 1: Title quality
  if (!postData.title || postData.title.length < 10) {
    results.checks.push({
      name: 'Title length',
      status: 'FAIL',
      message: 'Title must be at least 10 characters'
    });
    results.passed = false;
  } else if (postData.title.length > 100) {
    results.checks.push({
      name: 'Title length',
      status: 'WARN',
      message: 'Title is quite long (>100 chars)'
    });
    results.warnings.push('Consider a shorter, punchier title');
  } else {
    results.checks.push({
      name: 'Title length',
      status: 'PASS',
      message: `${postData.title.length} characters`
    });
  }

  // Check 2: Content length (strip HTML for count)
  const textContent = postData.content.replace(/<[^>]*>/g, '');
  const wordCount = textContent.split(/\s+/).filter(w => w.length > 0).length;

  if (wordCount < 300) {
    results.checks.push({
      name: 'Content length',
      status: 'FAIL',
      message: `Only ${wordCount} words. Minimum 300 required.`
    });
    results.passed = false;
  } else if (wordCount < 500) {
    results.checks.push({
      name: 'Content length',
      status: 'WARN',
      message: `${wordCount} words (recommended: 500+ for income strategies)`
    });
  } else {
    results.checks.push({
      name: 'Content length',
      status: 'PASS',
      message: `${wordCount} words`
    });
  }

  // Check 3: Structure indicators
  const hasHeadings = /<h[23][^>]*>/i.test(postData.content);
  const hasParagraphs = (postData.content.match(/<p>/gi) || []).length >= 3;

  if (!hasParagraphs) {
    results.checks.push({
      name: 'Structure',
      status: 'FAIL',
      message: 'Content needs at least 3 paragraphs'
    });
    results.passed = false;
  } else {
    results.checks.push({
      name: 'Structure',
      status: 'PASS',
      message: hasHeadings ? 'Has headings and paragraphs' : 'Has paragraph structure'
    });
  }

  // Check 4: No mid detection
  const genericPhrases = [
    /in today's world/i,
    /it is important to note/i,
    /as we all know/i,
    /this is a game changer/i,
    /moving forward/i,
    /at the end of the day/i
  ];

  const foundGeneric = genericPhrases.filter(p => p.test(textContent));
  if (foundGeneric.length > 2) {
    results.checks.push({
      name: 'No-mid check',
      status: 'WARN',
      message: `Found ${foundGeneric.length} generic phrases`
    });
    results.warnings.push('Remove corporate speak / generic phrasing');
  } else {
    results.checks.push({
      name: 'No-mid check',
      status: 'PASS',
      message: 'No excessive generic phrasing detected'
    });
  }

  // Summary
  results.summary = results.passed 
    ? (results.warnings.length > 0 ? 'PASS_WITH_WARNINGS' : 'PASS')
    : 'FAIL';

  return results;
}

function main() {
  const args = process.argv.slice(2);
  if (args.length === 0) {
    console.error('Usage: node quality-check.js \'{"title":"...","content":"..."}\'');
    process.exit(1);
  }

  let postData;
  try {
    postData = JSON.parse(args[0]);
  } catch (err) {
    console.error('Error parsing post data:', err.message);
    process.exit(1);
  }

  const results = qualityCheck(postData);

  // Output results
  console.log('\n=== Quality Check Results ===\n');
  
  results.checks.forEach(check => {
    const icon = check.status === 'PASS' ? '✓' : check.status === 'WARN' ? '!' : '✗';
    console.log(`${icon} ${check.name}: ${check.message}`);
  });

  if (results.warnings.length > 0) {
    console.log('\nWarnings:');
    results.warnings.forEach(w => console.log(`  ! ${w}`));
  }

  console.log(`\nOverall: ${results.summary}`);
  
  process.exit(results.passed ? 0 : 1);
}

main();
