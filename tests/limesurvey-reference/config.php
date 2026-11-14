<?php
if (!defined('BASEPATH') || getenv('RUSTSURVEY_REFERENCE_ONLY') !== '1') {
    throw new RuntimeException('This configuration is only for a disposable reference container.');
}

return [
    'components' => [
        'db' => [
            'connectionString' => 'mysql:host=database;dbname=rustsurvey_reference',
            'username' => 'reference',
            'password' => 'synthetic-reference-password',
            'charset' => 'utf8mb4',
            'tablePrefix' => 'lime_',
            'emulatePrepare' => true,
        ],
        'urlManager' => [
            'urlFormat' => 'get',
            'showScriptName' => true,
        ],
    ],
    'config' => [
        'debug' => 0,
        'debugsql' => 0,
        'defaultlang' => 'en',
        'forcehttps' => 'off',
        'updatecheckperiod' => 0,
    ],
];
