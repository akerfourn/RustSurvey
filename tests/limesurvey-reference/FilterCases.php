<?php
/** Compare row relevance produced by the pinned expression manager on synthetic source values. */
class FilterCases
{
    public static function run($directory, $output)
    {
        $bytes = file_get_contents($directory . '/filter-cases.json');
        if ($bytes === false || strlen($bytes) > 1048576) {
            throw new RuntimeException('Reference filter manifest is missing or too large.');
        }
        $cases = json_decode($bytes, true, 64, JSON_THROW_ON_ERROR);
        if (count($cases) > 64) {
            throw new RuntimeException('Reference filter corpus is too large.');
        }
        $results = [];
        foreach ($cases as $index => $case) {
            if ($case['file'] !== 'filters-' . $index . '.lss') {
                throw new RuntimeException('Invalid synthetic filter fixture path.');
            }
            $import = XMLImportSurvey($directory . '/' . $case['file']);
            if (!empty($import['error']) || empty($import['newsid'])) {
                throw new RuntimeException('Reference filter import failed.');
            }
            $surveyId = (int) $import['newsid'];
            $survey = Survey::model()->findByPk($surveyId);
            $target = Question::model()->findByAttributes(['sid' => $surveyId, 'title' => 'TARGET', 'parent_qid' => 0]);
            if (!$target) {
                throw new RuntimeException('Reference filter target is missing.');
            }
            $fields = [];
            foreach (createFieldMap($survey, 'full', true, false, 'fr') as $field) {
                if (!empty($field['qid'])) {
                    $fields[viewHelper::getFieldCode($field, ['LEMcompat' => true])] = $field['fieldname'];
                }
            }
            $_SESSION['LEMsid'] = $surveyId;
            $_SESSION['LEMlang'] = 'fr';
            $_SESSION['survey_' . $surveyId] = ['s_lang' => 'fr', 'token' => ''];
            LimeExpressionManager::StartSurvey($surveyId, 'survey', ['active' => false, 'anonymized' => true], true);
            LimeExpressionManager::StartProcessingPage(true, true);
            foreach ($fields as $alias => $field) {
                $value = $case['answers'][$alias] ?? '';
                $_SESSION['survey_' . $surveyId][$field] = $value;
                LimeExpressionManager::setValueToKnowVar($field, $value);
            }
            $manager = LimeExpressionManager::singleton();
            $manager->ProcessAllNeededRelevance();
            foreach ($case['answers'] as $alias => $value) {
                if (!isset($fields[$alias]) || (string) LimeExpressionManager::GetVarAttribute($fields[$alias], 'code', '', -1, -1) !== (string) $value) {
                    throw new RuntimeException('Reference filter source binding failed.');
                }
            }
            $manager->_CreateSubQLevelRelevanceAndValidationEqns();
            $property = new ReflectionProperty(LimeExpressionManager::class, 'subQrelInfo');
            $rules = $property->getValue($manager)[$target->qid] ?? [];
            $active = [];
            foreach ($case['expected'] as $alias => $_expected) {
                if (!isset($fields[$alias])) {
                    throw new RuntimeException('Reference filter field is missing.');
                }
                $rule = $rules[$fields[$alias]] ?? null;
                if ($rule !== null && $rule['hasErrors']) {
                    throw new RuntimeException('Reference filter expression has errors.');
                }
                $active[$alias] = $rule === null || (bool) $rule['result'];
            }
            $choices = null;
            if (isset($case['choices'])) {
                $choices = [];
                $prefix = $surveyId . 'X' . $target->gid . 'X' . $target->qid;
                foreach ($case['choices'] as $code => $_expected) {
                    $rule = $rules[$prefix . $code] ?? null;
                    if ($rule !== null && $rule['hasErrors']) {
                        throw new RuntimeException('Reference choice filter expression has errors.');
                    }
                    $choices[$code] = $rule === null || (bool) $rule['result'];
                }
            }
            $mandatoryViolation = null;
            if (isset($case['mandatoryViolation'])) {
                $status = $manager->_ValidateQuestion(LimeExpressionManager::GetQuestionSeq($target->qid));
                $mandatoryViolation = (bool) $status['mandViolation'];
            }
            $results[] = ['name' => $case['name'], 'active' => (object) $active, 'choices' => $choices, 'mandatoryViolation' => $mandatoryViolation, 'ruleIds' => array_keys($rules)];
        }
        if (file_put_contents($output . '/filter-results.json', json_encode($results, JSON_THROW_ON_ERROR)) === false) {
            throw new RuntimeException('Reference filter results could not be written.');
        }
    }
}
