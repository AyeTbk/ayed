fn void test_math() @test {
    int x = 10;
    int y = 20;
    test::eq(x + y, 40);
    // Test failed ^^^ ( example.c3:4 ) `30` != `40`
}